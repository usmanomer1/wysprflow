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
pub fn check_permissions() -> PermissionStatus {
    PermissionStatus {
        microphone: permissions::microphone(),
        accessibility: permissions::accessibility(),
        input_monitoring: permissions::input_monitoring(),
    }
}

#[tauri::command]
pub async fn request_microphone() -> bool {
    use cpal::traits::HostTrait;
    cpal::default_host().default_input_device().is_some()
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
