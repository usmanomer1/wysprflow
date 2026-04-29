pub mod keychain;

use std::path::PathBuf;
use std::sync::RwLock;

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationConfig {
    pub stt_provider: String,
    pub llm_provider: String,
    pub llm_model: String,
    pub hotkey: String,
    pub hotkey_mode: String,
    pub language: String,
    pub auto_cleanup: String,
    #[serde(default = "default_microphone_device")]
    pub microphone_device: String,
    #[serde(default = "default_preserve_clipboard")]
    pub preserve_clipboard: bool,
    #[serde(default)]
    pub play_sounds: bool,
    #[serde(default = "default_ide_file_tagging")]
    pub ide_file_tagging: bool,
    #[serde(default = "default_translate_to")]
    pub translate_to: String,
    #[serde(default = "default_snippets_enabled")]
    pub snippets_enabled: bool,
    #[serde(default)]
    pub custom_cleanup_prompt: String,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default)]
    pub setup_completed: bool,
}

impl Default for DictationConfig {
    fn default() -> Self {
        Self {
            stt_provider: "deepgram".into(),
            llm_provider: "anthropic".into(),
            llm_model: "claude-haiku-4-5".into(),
            hotkey: "CmdOrCtrl+Shift+Space".into(),
            hotkey_mode: "hold".into(),
            language: "auto".into(),
            auto_cleanup: "medium".into(),
            microphone_device: default_microphone_device(),
            preserve_clipboard: default_preserve_clipboard(),
            play_sounds: false,
            ide_file_tagging: default_ide_file_tagging(),
            translate_to: default_translate_to(),
            snippets_enabled: default_snippets_enabled(),
            custom_cleanup_prompt: String::new(),
            launch_at_login: false,
            setup_completed: false,
        }
    }
}

fn default_microphone_device() -> String {
    "default".into()
}

fn default_preserve_clipboard() -> bool {
    true
}

fn default_translate_to() -> String {
    "same".into()
}

fn default_ide_file_tagging() -> bool {
    true
}

fn default_snippets_enabled() -> bool {
    true
}

static STORE: OnceCell<RwLock<DictationConfig>> = OnceCell::new();
static CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();

pub fn initialize(app: &AppHandle) -> Result<()> {
    let dir = app.path().app_config_dir().context("app_config_dir")?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.json");
    let mut config: DictationConfig = if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&path)?).unwrap_or_default()
    } else {
        let cfg = DictationConfig::default();
        std::fs::write(&path, serde_json::to_string_pretty(&cfg)?)?;
        cfg
    };
    STORE
        .set(RwLock::new(config.clone()))
        .map_err(|_| anyhow::anyhow!("settings already initialized"))?;
    CONFIG_PATH
        .set(path)
        .map_err(|_| anyhow::anyhow!("config path already set"))?;
    keychain::initialize_validation_store(app)?;
    if config.setup_completed && !setup_requirements_satisfied(&config)? {
        config.setup_completed = false;
        if let Some(store) = STORE.get() {
            *store.write().unwrap() = config.clone();
        }
        if let Some(path) = CONFIG_PATH.get() {
            std::fs::write(path, serde_json::to_string_pretty(&config)?)?;
        }
    }
    Ok(())
}

pub fn get() -> DictationConfig {
    STORE
        .get()
        .expect("settings not initialized")
        .read()
        .unwrap()
        .clone()
}

pub fn update(patch: serde_json::Value) -> Result<DictationConfig> {
    let store = STORE.get().context("settings not initialized")?;
    let path = CONFIG_PATH.get().context("path not set")?;
    let mut current = store.write().unwrap();
    let merged = merge(&serde_json::to_value(&*current)?, &patch);
    let next: DictationConfig = serde_json::from_value(merged)?;
    *current = next.clone();
    std::fs::write(path, serde_json::to_string_pretty(&next)?)?;
    Ok(next)
}

fn merge(base: &serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            let mut out = b.clone();
            for (k, v) in p {
                let merged = merge(out.get(k).unwrap_or(&Value::Null), v);
                out.insert(k.clone(), merged);
            }
            Value::Object(out)
        }
        (_, p) => p.clone(),
    }
}

fn setup_requirements_satisfied(cfg: &DictationConfig) -> Result<bool> {
    let deepgram_ready = keychain::status("deepgram")?.has_key;
    if !deepgram_ready {
        return Ok(false);
    }

    let cleanup_ready = match cfg.llm_provider.as_str() {
        "openrouter" => keychain::status("openrouter")?.has_key,
        "anthropic" => keychain::status("anthropic")?.has_key,
        _ => true,
    };

    Ok(cleanup_ready)
}
