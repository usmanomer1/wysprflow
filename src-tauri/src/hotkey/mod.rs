// Global hotkey registration.
//
// We support two backends in parallel:
//   - tauri-plugin-global-shortcut for any standard Cmd/Ctrl/Alt/Shift+key combo.
//   - A CGEventTap-based Fn-key listener (`fn_key`) on macOS, since Carbon can't
//     capture Fn alone.
//
// Both dispatch into the same Pipeline state machine.

pub mod fn_key;

use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::{error, info, warn};

use crate::pipeline::Pipeline;

static REGISTERED: OnceLock<parking_lot::Mutex<Option<Shortcut>>> = OnceLock::new();

pub fn initialize(app: &AppHandle) -> Result<()> {
    REGISTERED.set(parking_lot::Mutex::new(None)).ok();
    let cfg = crate::settings::get();
    if let Err(e) = register(app, &cfg.hotkey) {
        warn!(
            "could not register configured hotkey '{}': {}",
            cfg.hotkey, e
        );
        let fallback = "CmdOrCtrl+Shift+Space";
        if let Err(e) = register(app, fallback) {
            error!("fallback hotkey registration failed: {}", e);
        }
    }

    // The Fn key listener runs in parallel — gives us the Wispr-style hold-Fn UX
    // even though Carbon can't see Fn directly.
    if let Err(e) = fn_key::install(app.clone()) {
        warn!("fn-key listener didn't install: {}", e);
    }

    Ok(())
}

pub fn set_hotkey(app: &AppHandle, accelerator: &str) -> Result<()> {
    if let Some(slot) = REGISTERED.get() {
        if let Some(prev) = slot.lock().take() {
            let _ = app.global_shortcut().unregister(prev);
        }
    }
    register(app, accelerator)
}

fn register(app: &AppHandle, accelerator: &str) -> Result<()> {
    let shortcut = parse_shortcut(accelerator)?;
    let app_for_handler = app.clone();

    app.global_shortcut()
        .on_shortcut(shortcut.clone(), move |_app, _shortcut, event| {
            let app = app_for_handler.clone();
            match event.state() {
                ShortcutState::Pressed => {
                    tauri::async_runtime::spawn(async move {
                        if let Some(pipeline) = app.try_state::<Pipeline>() {
                            if let Err(e) = pipeline.start() {
                                error!("pipeline start failed: {}", e);
                            }
                        }
                    });
                }
                ShortcutState::Released => {
                    tauri::async_runtime::spawn(async move {
                        if let Some(pipeline) = app.try_state::<Pipeline>() {
                            if let Err(e) = pipeline.stop().await {
                                error!("pipeline stop failed: {}", e);
                            }
                        }
                    });
                }
            }
        })
        .with_context(|| format!("on_shortcut({})", accelerator))?;

    if let Some(slot) = REGISTERED.get() {
        *slot.lock() = Some(shortcut);
    }
    info!("hotkey registered: {}", accelerator);
    Ok(())
}

fn parse_shortcut(s: &str) -> Result<Shortcut> {
    let mut modifiers = Modifiers::empty();
    let mut code: Option<Code> = None;
    for part in s.split('+') {
        let lower = part.trim().to_ascii_lowercase();
        match lower.as_str() {
            "" => {}
            "cmd" | "command" | "meta" | "super" | "win" | "windows" => {
                modifiers |= Modifiers::SUPER
            }
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" | "opt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "cmdorctrl" | "controlorcommand" | "controlorcmd" => {
                #[cfg(target_os = "macos")]
                {
                    modifiers |= Modifiers::SUPER;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    modifiers |= Modifiers::CONTROL;
                }
            }
            other => {
                code = Some(parse_code(other).ok_or_else(|| anyhow!("unknown key '{}'", part))?);
            }
        }
    }
    let code = code.ok_or_else(|| anyhow!("no key in accelerator '{}'", s))?;
    Ok(Shortcut::new(Some(modifiers), code))
}

fn parse_code(s: &str) -> Option<Code> {
    Some(match s {
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "escape" | "esc" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        _ => return None,
    })
}
