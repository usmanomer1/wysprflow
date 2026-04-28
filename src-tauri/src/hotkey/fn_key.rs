// CGEventTap-based listener for the Fn (Globe) key on macOS.
//
// The Carbon hotkey API used by tauri-plugin-global-shortcut cannot capture the bare Fn
// modifier — it's not a regular keystroke and has no key code in the legacy API. The
// only reliable path is a CGEventTap on `kCGEventFlagsChanged`, watching for the
// `NSEventModifierFlagFunction` (0x800000) bit on the modifier flags.
//
// Requires the "Input Monitoring" permission. If it's denied, CGEventTap creation
// fails and the listener exits gracefully — global-shortcut continues to work as a
// fallback for Cmd/Ctrl/Alt/Shift combos.

use anyhow::Result;
use tauri::AppHandle;

#[cfg(target_os = "macos")]
pub fn install(app: AppHandle) -> Result<()> {
    use std::thread;
    thread::Builder::new()
        .name("wysprflow-fnkey".into())
        .spawn(move || {
            run_tap(app);
        })
        .map_err(|e| anyhow::anyhow!("spawn fn-key thread: {}", e))?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn install(_app: AppHandle) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_tap(app: AppHandle) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use core_foundation::base::TCFType;
    use core_foundation::runloop::{
        kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRun,
    };
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    use tauri::Manager;
    use tracing::{error, info, warn};

    use crate::pipeline::Pipeline;

    const FN_FLAG: u64 = 0x800000; // NSEventModifierFlagFunction

    let last_fn = Arc::new(AtomicBool::new(false));
    let app_for_cb = app.clone();
    let last_fn_cb = last_fn.clone();

    let tap = match CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::FlagsChanged],
        move |_proxy, _kind, event| {
            let flags = event.get_flags().bits();
            let fn_now = (flags & FN_FLAG) != 0;
            let prev = last_fn_cb.load(Ordering::Relaxed);
            if fn_now != prev {
                last_fn_cb.store(fn_now, Ordering::Relaxed);
                let app = app_for_cb.clone();
                if fn_now {
                    tauri::async_runtime::spawn(async move {
                        if let Some(p) = app.try_state::<Pipeline>() {
                            if let Err(e) = p.start() {
                                error!("fn-press pipeline start failed: {}", e);
                            }
                        }
                    });
                } else {
                    tauri::async_runtime::spawn(async move {
                        if let Some(p) = app.try_state::<Pipeline>() {
                            if let Err(e) = p.stop().await {
                                error!("fn-release pipeline stop failed: {}", e);
                            }
                        }
                    });
                }
            }
            Some(event.clone())
        },
    ) {
        Ok(t) => t,
        Err(_) => {
            warn!("CGEventTap creation failed — Input Monitoring permission likely not granted. Fn-key support disabled; global-shortcut combos still work.");
            return;
        }
    };

    let source = match tap.mach_port.create_runloop_source(0) {
        Ok(s) => s,
        Err(_) => {
            warn!("create_runloop_source failed");
            return;
        }
    };

    unsafe {
        CFRunLoopAddSource(
            CFRunLoopGetCurrent(),
            source.as_concrete_TypeRef(),
            kCFRunLoopCommonModes,
        );
    }
    tap.enable();

    info!("fn-key tap installed");

    unsafe {
        CFRunLoopRun();
    }
}
