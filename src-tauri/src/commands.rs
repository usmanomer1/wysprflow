use serde::Serialize;
use tauri::AppHandle;

use crate::audio::capture::AudioInputDevice;
use crate::ax::permissions;
use crate::db::Db;
use crate::dictionary::{self, DictEntry};
use crate::history::{self, HistoryEntry};
use crate::llm::anthropic::AnthropicClient;
use crate::llm::openrouter::OpenRouterClient;
use crate::pipeline::Pipeline;
use crate::settings::{self, keychain, DictationConfig};
use crate::snippets::{self, Snippet};
use crate::startup;
use crate::stt::deepgram::DeepgramClient;

#[tauri::command]
pub fn get_config() -> DictationConfig {
    settings::get()
}

#[tauri::command]
pub fn update_config(_app: AppHandle, patch: serde_json::Value) -> Result<DictationConfig, String> {
    let previous = settings::get();
    let next = settings::update(patch).map_err(|e| e.to_string())?;
    if previous.launch_at_login != next.launch_at_login {
        if let Err(e) = startup::set_launch_at_login(next.launch_at_login) {
            let rollback = serde_json::to_value(&previous).map_err(|err| err.to_string())?;
            let _ = settings::update(rollback);
            return Err(e.to_string());
        }
    }
    Ok(next)
}

#[tauri::command]
pub fn get_api_key_status(provider: String) -> Result<keychain::ApiKeyStatus, String> {
    keychain::status(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_api_key_statuses() -> Result<Vec<keychain::ApiKeyStatus>, String> {
    keychain::list_status().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_api_key(provider: String, key: String) -> Result<keychain::ApiKeyStatus, String> {
    keychain::set(&provider, &key).map_err(|e| e.to_string())?;
    keychain::status(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    keychain::delete(&provider).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct ValidateResult {
    pub ok: bool,
    pub detail: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationStatus {
    pub bundle_path: String,
    pub in_applications: bool,
    pub is_translocated: bool,
}

#[tauri::command]
pub async fn validate_api_key(provider: String) -> Result<ValidateResult, String> {
    let key = keychain::get(&provider).map_err(|e| e.to_string())?;
    let key = match key {
        Some(k) => k,
        None => {
            return Ok(ValidateResult {
                ok: false,
                detail: Some("No key stored".into()),
            })
        }
    };
    let result = match provider.as_str() {
        "anthropic" => AnthropicClient::new(key).validate().await,
        "openrouter" => OpenRouterClient::new(key).validate().await,
        "deepgram" => DeepgramClient::new(key).validate().await,
        _ => Err(anyhow::anyhow!(
            "Validation is not implemented for {}",
            provider
        )),
    };
    match result {
        Ok(()) => {
            keychain::mark_validated(&provider).ok();
            Ok(ValidateResult {
                ok: true,
                detail: None,
            })
        }
        Err(e) => Ok(ValidateResult {
            ok: false,
            detail: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn list_audio_input_devices() -> Result<Vec<AudioInputDevice>, String> {
    crate::audio::capture::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, accelerator: String) -> Result<(), String> {
    crate::hotkey::set_hotkey(&app, &accelerator).map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub microphone: permissions::PermissionState,
    pub accessibility: permissions::PermissionState,
    pub input_monitoring: permissions::PermissionState,
}

#[tauri::command]
pub fn check_permissions(app: AppHandle) -> PermissionStatus {
    let _ = crate::hotkey::fn_key::install(app);
    PermissionStatus {
        microphone: permissions::microphone(),
        accessibility: permissions::accessibility(),
        input_monitoring: permissions::input_monitoring(),
    }
}

#[tauri::command]
pub fn get_installation_status() -> InstallationStatus {
    let exe = std::env::current_exe().unwrap_or_default();
    let mut bundle_path = exe.clone();

    for ancestor in exe.ancestors() {
        if ancestor.extension().and_then(|ext| ext.to_str()) == Some("app") {
            bundle_path = ancestor.to_path_buf();
            break;
        }
    }

    let bundle_display = bundle_path.to_string_lossy().into_owned();
    let home_applications = std::env::var("HOME")
        .ok()
        .map(|home| format!("{home}/Applications/"))
        .unwrap_or_default();

    InstallationStatus {
        in_applications: bundle_display.starts_with("/Applications/")
            || (!home_applications.is_empty() && bundle_display.starts_with(&home_applications)),
        is_translocated: bundle_display.contains("/AppTranslocation/"),
        bundle_path: bundle_display,
    }
}

#[tauri::command]
pub fn move_to_applications(app: AppHandle) -> Result<(), String> {
    let status = get_installation_status();
    if status.in_applications && !status.is_translocated {
        return Ok(());
    }

    let source = std::path::PathBuf::from(&status.bundle_path);
    if source.extension().and_then(|ext| ext.to_str()) != Some("app") {
        return Err("Could not locate running app bundle".into());
    }

    let product_name = "wysprflow.app";
    let mut targets = vec![std::path::PathBuf::from("/Applications").join(product_name)];
    if let Ok(home) = std::env::var("HOME") {
        targets.push(
            std::path::PathBuf::from(home)
                .join("Applications")
                .join(product_name),
        );
    }

    let mut last_error = None;
    for target in targets {
        if let Some(parent) = target.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                last_error = Some(e.to_string());
                continue;
            }
        }

        if target != source && target.exists() {
            if let Err(e) = std::fs::remove_dir_all(&target) {
                last_error = Some(e.to_string());
                continue;
            }
        }

        let status = std::process::Command::new("/usr/bin/ditto")
            .arg(&source)
            .arg(&target)
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            last_error = Some(format!("ditto failed for {}", target.display()));
            continue;
        }

        std::process::Command::new("open")
            .arg("-n")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;

        app.exit(0);
        return Ok(());
    }

    Err(last_error.unwrap_or_else(|| "Couldn't move app into Applications".into()))
}

#[tauri::command]
pub async fn request_microphone() -> bool {
    let (audio_tx, _audio_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<f32>>();
    match crate::audio::capture::start(None, audio_tx) {
        Ok(handle) => {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            drop(handle);
            true
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    permissions::open_accessibility_settings();
    Ok(())
}

#[tauri::command]
pub fn open_input_monitoring_settings() -> Result<(), String> {
    permissions::open_input_monitoring_settings();
    Ok(())
}

#[tauri::command]
pub fn request_accessibility() -> PermissionStatus {
    let accessibility = permissions::request_accessibility();
    PermissionStatus {
        microphone: permissions::microphone(),
        accessibility,
        input_monitoring: permissions::input_monitoring(),
    }
}

#[tauri::command]
pub fn open_microphone_settings() -> Result<(), String> {
    permissions::open_microphone_settings();
    Ok(())
}

#[tauri::command]
pub fn start_dictation(pipeline: tauri::State<'_, Pipeline>) -> Result<(), String> {
    pipeline.start().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_dictation(pipeline: tauri::State<'_, Pipeline>) -> Result<(), String> {
    pipeline.stop().await.map_err(|e| e.to_string())
}

// ---- Dictionary -----------------------------------------------------------

#[tauri::command]
pub fn list_dictionary(db: tauri::State<'_, Db>) -> Result<Vec<DictEntry>, String> {
    dictionary::list(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_dictionary_word(db: tauri::State<'_, Db>, word: String) -> Result<DictEntry, String> {
    dictionary::add(&db, &word).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_dictionary_word(db: tauri::State<'_, Db>, id: i64) -> Result<(), String> {
    dictionary::delete(&db, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_dictionary_star(db: tauri::State<'_, Db>, id: i64) -> Result<DictEntry, String> {
    dictionary::toggle_star(&db, id).map_err(|e| e.to_string())
}

// ---- Snippets -------------------------------------------------------------

#[tauri::command]
pub fn list_snippets(db: tauri::State<'_, Db>) -> Result<Vec<Snippet>, String> {
    snippets::list(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_snippet(
    db: tauri::State<'_, Db>,
    id: Option<i64>,
    trigger: String,
    expansion: String,
) -> Result<Snippet, String> {
    snippets::upsert(&db, id, &trigger, &expansion).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_snippet(db: tauri::State<'_, Db>, id: i64) -> Result<(), String> {
    snippets::delete(&db, id).map_err(|e| e.to_string())
}

// ---- History --------------------------------------------------------------

#[tauri::command]
pub fn list_history(
    db: tauri::State<'_, Db>,
    limit: Option<i64>,
) -> Result<Vec<HistoryEntry>, String> {
    history::list(&db, limit.unwrap_or(200)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_history(
    db: tauri::State<'_, Db>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<HistoryEntry>, String> {
    history::search(&db, &query, limit.unwrap_or(200)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history_entry(db: tauri::State<'_, Db>, id: String) -> Result<(), String> {
    history::delete(&db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(db: tauri::State<'_, Db>) -> Result<(), String> {
    history::clear_all(&db).map_err(|e| e.to_string())
}
