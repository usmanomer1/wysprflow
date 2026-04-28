// Floating HUD pill control.
//
// Phase 2: positions the HUD top-center of the focused screen on show. On macOS
// the window is configured `focus: false` in tauri.conf.json so it doesn't steal
// keyboard focus from the user's text field — that's what makes paste injection
// work without a true NSPanel promotion. (NSPanel polish is Phase 2.5.)

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudState {
    pub state: String,
    pub message: Option<String>,
    pub level: Option<f32>,
}

impl HudState {
    pub fn idle() -> Self {
        Self {
            state: "idle".into(),
            message: None,
            level: None,
        }
    }

    pub fn initializing() -> Self {
        Self {
            state: "initializing".into(),
            message: None,
            level: None,
        }
    }

    pub fn listening(level: f32) -> Self {
        Self {
            state: "listening".into(),
            message: None,
            level: Some(level),
        }
    }

    pub fn processing_with_message(msg: impl Into<String>) -> Self {
        Self {
            state: "processing".into(),
            message: Some(msg.into()),
            level: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            state: "error".into(),
            message: Some(msg.into()),
            level: None,
        }
    }
}

pub fn initialize(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

pub fn show(app: &AppHandle) -> tauri::Result<()> {
    match app.get_webview_window("hud") {
        Some(win) => {
            tracing::info!("hud: 'hud' window found, positioning + showing");
            if let Err(e) = position_top_center(&win) {
                warn!("hud positioning failed: {}", e);
            }
            win.show()?;
            tracing::info!("hud: shown");
        }
        None => {
            warn!("hud: 'hud' window NOT found in webview registry");
        }
    }
    Ok(())
}

pub fn hide(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("hud") {
        win.hide()?;
    }
    Ok(())
}

pub fn emit_state(app: &AppHandle, state: HudState) -> tauri::Result<()> {
    app.emit("hud-state", state)
}

fn position_top_center(win: &tauri::WebviewWindow) -> tauri::Result<()> {
    // Pick the monitor the cursor is currently on (so multi-monitor works).
    // Falls back to the window's current monitor, then the primary monitor.
    let monitor = win
        .cursor_position()
        .ok()
        .and_then(|pos| win.monitor_from_point(pos.x, pos.y).ok().flatten())
        .or_else(|| win.current_monitor().ok().flatten())
        .or_else(|| win.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return Ok(());
    };

    let outer = win.outer_size()?;
    let m_pos = monitor.position();
    let m_size = monitor.size();

    let x = m_pos.x + ((m_size.width as i32) - (outer.width as i32)) / 2;
    let y = m_pos.y; // flush with the top of the screen — sits beside/under the notch

    win.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}
