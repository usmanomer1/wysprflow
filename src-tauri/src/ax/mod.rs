// Text injection + macOS permissions.
//
// Phase 1b: clipboard-paste injection only.
//   1. Save current pasteboard contents.
//   2. Write our cleaned text to the pasteboard.
//   3. Synthesize Cmd+V via enigo (CGEvent under the hood on macOS).
//   4. After a short delay, restore the original pasteboard.
//
// Phase 2 added:
//   - Real AXIsProcessTrusted permission inspection (`permissions` module).
//   - Open-Settings deep links for Mic / Accessibility / Input Monitoring.
//
// Phase 2.5 will add AXUIElement direct insertion (paste-free) for compatible apps.

pub mod permissions;

use std::time::Duration;

use anyhow::{bail, Context, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tracing::{debug, warn};

/// Paste `text` into whatever app currently has keyboard focus. Optionally saves and
/// restores the user's clipboard around the operation.
pub async fn inject(app: &AppHandle, text: &str, preserve_clipboard: bool) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    if !matches!(
        permissions::accessibility(),
        permissions::PermissionState::Granted
    ) {
        bail!("Accessibility permission not granted");
    }

    debug!("inject: reading clipboard");
    let saved = if preserve_clipboard {
        app.clipboard().read_text().ok()
    } else {
        None
    };
    debug!("inject: writing transcript to clipboard");
    app.clipboard()
        .write_text(text)
        .context("write transcript to clipboard")?;

    tokio::time::sleep(Duration::from_millis(40)).await;

    debug!("inject: scheduling paste keystroke");
    tokio::time::timeout(
        Duration::from_secs(2),
        paste_keystroke_on_main_thread(app.clone()),
    )
    .await
    .context("paste keystroke timed out")?
    .context("synthesize paste")?;
    debug!("inject: paste keystroke finished");

    if let Some(prev) = saved {
        tokio::time::sleep(Duration::from_millis(180)).await;
        debug!("inject: restoring clipboard");
        if let Err(e) = app.clipboard().write_text(prev) {
            warn!("could not restore clipboard: {}", e);
        }
    }

    debug!("injected {} chars", text.len());
    Ok(())
}

async fn paste_keystroke_on_main_thread(app: AppHandle) -> Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel::<std::result::Result<(), String>>();
    app.run_on_main_thread(move || {
        let result = paste_keystroke().map_err(|e| e.to_string());
        let _ = tx.send(result);
    })
    .context("schedule paste on main thread")?;

    rx.await
        .context("main-thread paste callback dropped")?
        .map_err(anyhow::Error::msg)
}

fn paste_keystroke() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    let mod_key = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    enigo
        .key(mod_key, Direction::Press)
        .context("press paste modifier")?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .context("press V for paste")?;
    enigo
        .key(mod_key, Direction::Release)
        .context("release paste modifier")?;
    Ok(())
}
