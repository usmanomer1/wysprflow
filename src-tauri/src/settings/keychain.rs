// API key storage.
//
// Release macOS builds use Keychain.
// Debug macOS builds use a local JSON file in app_config_dir so `tauri dev`
// doesn't trigger repeated Keychain password prompts from the unsigned/dev binary.
//
// Validation timestamps live in a tiny JSON file alongside config.json,
// since neither backend tracks when we last saw the provider succeed.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use anyhow::{Context, Result};
use chrono::Utc;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[cfg(all(target_os = "macos", not(debug_assertions)))]
const SERVICE: &str = "app.wysprflow";

pub const PROVIDERS: &[&str] = &[
    "anthropic",
    "openrouter",
    "deepgram",
    "groq",
    "openai",
    "elevenlabs",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub provider: String,
    pub has_key: bool,
    pub validated_at: Option<String>,
}

static VALIDATIONS: OnceCell<RwLock<HashMap<String, String>>> = OnceCell::new();
static VALIDATIONS_PATH: OnceCell<PathBuf> = OnceCell::new();
static CACHE: OnceCell<RwLock<HashMap<String, Option<String>>>> = OnceCell::new();
#[cfg(all(target_os = "macos", debug_assertions))]
static DEV_STORE: OnceCell<RwLock<HashMap<String, String>>> = OnceCell::new();
#[cfg(all(target_os = "macos", debug_assertions))]
static DEV_STORE_PATH: OnceCell<PathBuf> = OnceCell::new();

pub fn initialize_validation_store(app: &AppHandle) -> Result<()> {
    let dir = app.path().app_config_dir().context("app_config_dir")?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("validations.json");
    let map: HashMap<String, String> = if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&path)?).unwrap_or_default()
    } else {
        HashMap::new()
    };
    VALIDATIONS.set(RwLock::new(map)).ok();
    VALIDATIONS_PATH.set(path).ok();
    CACHE.set(RwLock::new(HashMap::new())).ok();
    #[cfg(all(target_os = "macos", debug_assertions))]
    {
        let path = dir.join("dev-secrets.json");
        let map: HashMap<String, String> = if path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&path)?).unwrap_or_default()
        } else {
            HashMap::new()
        };
        DEV_STORE.set(RwLock::new(map)).ok();
        DEV_STORE_PATH.set(path).ok();
    }
    Ok(())
}

fn cache_get(provider: &str) -> Option<Option<String>> {
    CACHE.get()?.read().ok()?.get(provider).cloned()
}

fn cache_put(provider: &str, value: Option<String>) {
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    cache.write().unwrap().insert(provider.to_string(), value);
}

fn write_validations() -> Result<()> {
    let map = VALIDATIONS.get().context("validations not initialized")?;
    let path = VALIDATIONS_PATH.get().context("path not set")?;
    let m = map.read().unwrap();
    std::fs::write(path, serde_json::to_string_pretty(&*m)?)?;
    Ok(())
}

#[cfg(all(target_os = "macos", debug_assertions))]
fn write_dev_store() -> Result<()> {
    let store = DEV_STORE.get().context("dev store not initialized")?;
    let path = DEV_STORE_PATH.get().context("dev store path not set")?;
    let m = store.read().unwrap();
    std::fs::write(path, serde_json::to_string_pretty(&*m)?)?;
    Ok(())
}

pub fn mark_validated(provider: &str) -> Result<()> {
    let map = VALIDATIONS.get().context("validations not initialized")?;
    map.write()
        .unwrap()
        .insert(provider.to_string(), Utc::now().to_rfc3339());
    write_validations()
}

fn validated_at(provider: &str) -> Option<String> {
    VALIDATIONS.get()?.read().ok()?.get(provider).cloned()
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
fn exists(provider: &str) -> Result<bool> {
    use security_framework::item::{ItemClass, ItemSearchOptions};

    match ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(SERVICE)
        .account(provider)
        .search()
    {
        Ok(_) => Ok(true),
        Err(e) if e.code() == -25300 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

#[cfg(all(target_os = "macos", debug_assertions))]
fn exists(provider: &str) -> Result<bool> {
    Ok(DEV_STORE
        .get()
        .context("dev store not initialized")?
        .read()
        .unwrap()
        .contains_key(provider))
}

#[cfg(not(target_os = "macos"))]
fn exists(_provider: &str) -> Result<bool> {
    Ok(false)
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
pub fn set(provider: &str, key: &str) -> Result<()> {
    use security_framework::passwords::set_generic_password;
    set_generic_password(SERVICE, provider, key.as_bytes()).context("Keychain write failed")?;
    cache_put(provider, Some(key.to_string()));
    // Saving a new key invalidates prior validation
    if let Some(map) = VALIDATIONS.get() {
        map.write().unwrap().remove(provider);
        let _ = write_validations();
    }
    Ok(())
}

#[cfg(all(target_os = "macos", debug_assertions))]
pub fn set(provider: &str, key: &str) -> Result<()> {
    let store = DEV_STORE.get().context("dev store not initialized")?;
    store
        .write()
        .unwrap()
        .insert(provider.to_string(), key.to_string());
    write_dev_store()?;
    cache_put(provider, Some(key.to_string()));
    if let Some(map) = VALIDATIONS.get() {
        map.write().unwrap().remove(provider);
        let _ = write_validations();
    }
    Ok(())
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
pub fn get(provider: &str) -> Result<Option<String>> {
    if let Some(value) = cache_get(provider) {
        return Ok(value);
    }

    use security_framework::passwords::get_generic_password;
    match get_generic_password(SERVICE, provider) {
        Ok(bytes) => {
            let key = String::from_utf8(bytes)?;
            cache_put(provider, Some(key.clone()));
            Ok(Some(key))
        }
        Err(e) => {
            // errSecItemNotFound = -25300
            if e.code() == -25300 {
                cache_put(provider, None);
                Ok(None)
            } else {
                Err(e.into())
            }
        }
    }
}

#[cfg(all(target_os = "macos", debug_assertions))]
pub fn get(provider: &str) -> Result<Option<String>> {
    if let Some(value) = cache_get(provider) {
        return Ok(value);
    }

    let value = DEV_STORE
        .get()
        .context("dev store not initialized")?
        .read()
        .unwrap()
        .get(provider)
        .cloned();
    cache_put(provider, value.clone());
    Ok(value)
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
pub fn delete(provider: &str) -> Result<()> {
    use security_framework::passwords::delete_generic_password;
    match delete_generic_password(SERVICE, provider) {
        Ok(_) => {
            cache_put(provider, None);
            if let Some(map) = VALIDATIONS.get() {
                map.write().unwrap().remove(provider);
                let _ = write_validations();
            }
            Ok(())
        }
        Err(e) if e.code() == -25300 => {
            cache_put(provider, None);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(all(target_os = "macos", debug_assertions))]
pub fn delete(provider: &str) -> Result<()> {
    let store = DEV_STORE.get().context("dev store not initialized")?;
    store.write().unwrap().remove(provider);
    write_dev_store()?;
    cache_put(provider, None);
    if let Some(map) = VALIDATIONS.get() {
        map.write().unwrap().remove(provider);
        let _ = write_validations();
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn set(_provider: &str, _key: &str) -> Result<()> {
    anyhow::bail!("Keychain not implemented for this platform yet")
}

#[cfg(not(target_os = "macos"))]
pub fn get(_provider: &str) -> Result<Option<String>> {
    Ok(None)
}

#[cfg(not(target_os = "macos"))]
pub fn delete(_provider: &str) -> Result<()> {
    Ok(())
}

pub fn status(provider: &str) -> Result<ApiKeyStatus> {
    let has_key = match cache_get(provider) {
        Some(value) => value.is_some(),
        None => exists(provider)?,
    };
    Ok(ApiKeyStatus {
        provider: provider.to_string(),
        has_key,
        validated_at: if has_key {
            validated_at(provider)
        } else {
            None
        },
    })
}

pub fn list_status() -> Result<Vec<ApiKeyStatus>> {
    PROVIDERS.iter().map(|p| status(p)).collect()
}
