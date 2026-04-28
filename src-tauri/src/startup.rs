use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

const LABEL: &str = "com.wysprflow.desktop";

pub fn set_launch_at_login(enabled: bool) -> Result<()> {
    let path = plist_path()?;
    if enabled {
        write_launch_agent(&path)?;
    } else if path.exists() {
        fs::remove_file(&path).context("remove LaunchAgent plist")?;
    }
    Ok(())
}

fn plist_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let dir = PathBuf::from(home).join("Library/LaunchAgents");
    fs::create_dir_all(&dir).context("create LaunchAgents dir")?;
    Ok(dir.join(format!("{LABEL}.plist")))
}

fn write_launch_agent(path: &PathBuf) -> Result<()> {
    let exe = std::env::current_exe()
        .context("current_exe")?
        .canonicalize()
        .context("canonicalize current_exe")?;
    let working_dir = exe
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".into());
    let exe = exe.to_string_lossy();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <false/>
  <key>WorkingDirectory</key>
  <string>{working_dir}</string>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
"#,
        label = LABEL,
        exe = xml_escape(&exe),
        working_dir = xml_escape(&working_dir)
    );

    fs::write(path, plist).context("write LaunchAgent plist")?;
    Ok(())
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
