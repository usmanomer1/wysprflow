// macOS permission inspection. We surface three states to the UI:
//   - granted
//   - denied
//   - notDetermined  (system hasn't asked the user yet)
//
// Microphone is checked via cpal's ability to enumerate input devices (a heuristic
// good enough for "did the OS prompt fire"). A real AVCaptureDevice authorization
// status check would require obj-c bridging — added in a Phase 2.5 polish if needed.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionState {
    Granted,
    Denied,
    NotDetermined,
}

pub fn microphone() -> PermissionState {
    use cpal::traits::HostTrait;
    let host = cpal::default_host();
    match host.default_input_device() {
        Some(_) => PermissionState::Granted,
        None => PermissionState::NotDetermined,
    }
}

pub fn accessibility() -> PermissionState {
    #[cfg(target_os = "macos")]
    {
        if unsafe { ax_is_process_trusted() } {
            PermissionState::Granted
        } else {
            PermissionState::Denied
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionState::Granted
    }
}

pub fn input_monitoring() -> PermissionState {
    // Best heuristic without IOHIDCheckAccess bridging: if AX is granted, the user
    // has done the privileged-grant dance and Input Monitoring is usually granted
    // alongside. The Fn-key listener will log if it actually fails.
    accessibility()
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
unsafe fn ax_is_process_trusted() -> bool {
    AXIsProcessTrusted()
}

pub fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

pub fn open_input_monitoring_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn();
    }
}

pub fn open_microphone_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
            .spawn();
    }
}
