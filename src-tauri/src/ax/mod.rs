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

const CLIPBOARD_SETTLE_DELAY_MS: u64 = 40;
const CLIPBOARD_RESTORE_DELAY_MS: u64 = 180;
const MENTION_MENU_DELAY_MS: u64 = 120;
const MENTION_COMMIT_SETTLE_MS: u64 = 90;

#[derive(Debug)]
enum InjectChunk {
    Text(String),
    FileTag(String),
}

/// Paste `text` into whatever app currently has keyboard focus. Optionally saves and
/// restores the user's clipboard around the operation.
pub async fn inject(
    app: &AppHandle,
    text: &str,
    preserve_clipboard: bool,
    source_app: Option<&str>,
    file_tagging_enabled: bool,
) -> Result<()> {
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
    let used_file_tagging = if file_tagging_enabled && supports_chat_file_tagging(source_app) {
        let chunks = split_inject_chunks(text);
        if chunks
            .iter()
            .any(|chunk| matches!(chunk, InjectChunk::FileTag(_)))
        {
            debug!("inject: using Cursor/Windsurf file-tag flow");
            inject_chunks(app, &chunks).await?;
            true
        } else {
            false
        }
    } else {
        false
    };

    if !used_file_tagging {
        paste_text_fragment(app, text).await?;
    }

    if let Some(prev) = saved {
        tokio::time::sleep(Duration::from_millis(CLIPBOARD_RESTORE_DELAY_MS)).await;
        debug!("inject: restoring clipboard");
        if let Err(e) = app.clipboard().write_text(prev) {
            warn!("could not restore clipboard: {}", e);
        }
    }

    debug!("injected {} chars", text.len());
    Ok(())
}

async fn inject_chunks(app: &AppHandle, chunks: &[InjectChunk]) -> Result<()> {
    for chunk in chunks {
        match chunk {
            InjectChunk::Text(text) => {
                if !text.is_empty() {
                    paste_text_fragment(app, text).await?;
                }
            }
            InjectChunk::FileTag(query) => {
                debug!("inject: committing file tag {:?}", query);
                type_text_on_main_thread(app.clone(), "@")
                    .await
                    .context("type file-tag trigger")?;
                tokio::time::sleep(Duration::from_millis(CLIPBOARD_SETTLE_DELAY_MS)).await;
                type_text_on_main_thread(app.clone(), query)
                    .await
                    .context("type file-tag query")?;
                tokio::time::sleep(Duration::from_millis(MENTION_MENU_DELAY_MS)).await;
                tokio::time::timeout(
                    Duration::from_secs(2),
                    key_click_on_main_thread(app.clone(), Key::Return),
                )
                .await
                .context("mention commit timed out")?
                .context("confirm file mention")?;
                tokio::time::sleep(Duration::from_millis(MENTION_COMMIT_SETTLE_MS)).await;
            }
        }
    }
    Ok(())
}

async fn paste_text_fragment(app: &AppHandle, text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    debug!("inject: writing transcript to clipboard");
    app.clipboard()
        .write_text(text)
        .context("write transcript to clipboard")?;

    tokio::time::sleep(Duration::from_millis(CLIPBOARD_SETTLE_DELAY_MS)).await;

    debug!("inject: scheduling paste keystroke");
    tokio::time::timeout(
        Duration::from_secs(2),
        paste_keystroke_on_main_thread(app.clone()),
    )
    .await
    .context("paste keystroke timed out")?
    .context("synthesize paste")?;
    debug!("inject: paste keystroke finished");
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

async fn type_text_on_main_thread(app: AppHandle, text: &str) -> Result<()> {
    let text = text.to_string();
    let (tx, rx) = tokio::sync::oneshot::channel::<std::result::Result<(), String>>();
    app.run_on_main_thread(move || {
        let result = type_text(&text).map_err(|e| e.to_string());
        let _ = tx.send(result);
    })
    .context("schedule text entry on main thread")?;

    rx.await
        .context("main-thread text callback dropped")?
        .map_err(anyhow::Error::msg)
}

async fn key_click_on_main_thread(app: AppHandle, key: Key) -> Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel::<std::result::Result<(), String>>();
    app.run_on_main_thread(move || {
        let result = key_click(key).map_err(|e| e.to_string());
        let _ = tx.send(result);
    })
    .context("schedule key click on main thread")?;

    rx.await
        .context("main-thread key callback dropped")?
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

fn key_click(key: Key) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    enigo.key(key, Direction::Click).context("click key")?;
    Ok(())
}

fn type_text(text: &str) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    enigo.text(text).context("type text")?;
    Ok(())
}

fn supports_chat_file_tagging(source_app: Option<&str>) -> bool {
    let lower = source_app.unwrap_or_default().trim().to_ascii_lowercase();
    lower == "cursor" || lower == "windsurf"
}

fn split_inject_chunks(text: &str) -> Vec<InjectChunk> {
    let mut chunks = Vec::new();

    for segment in text.split_inclusive(char::is_whitespace) {
        split_segment_into_chunks(segment, &mut chunks);
    }

    if !text.chars().last().map(char::is_whitespace).unwrap_or(true) {
        let consumed = text
            .split_inclusive(char::is_whitespace)
            .map(str::len)
            .sum::<usize>();
        if consumed < text.len() {
            split_segment_into_chunks(&text[consumed..], &mut chunks);
        }
    }

    chunks
}

fn split_segment_into_chunks(segment: &str, chunks: &mut Vec<InjectChunk>) {
    if segment.is_empty() {
        return;
    }

    let trailing_ws_len = segment
        .chars()
        .rev()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let (body, trailing_ws) = if trailing_ws_len == 0 {
        (segment, "")
    } else {
        let split_at = segment.len() - trailing_ws_len;
        (&segment[..split_at], &segment[split_at..])
    };

    if body.is_empty() {
        push_text_chunk(chunks, trailing_ws.to_string());
        return;
    }

    let leading_len = body
        .chars()
        .take_while(|ch| !is_file_tag_leading_char(*ch))
        .map(char::len_utf8)
        .sum::<usize>();
    let trailing_len = body
        .chars()
        .rev()
        .take_while(|ch| !is_file_tag_trailing_char(*ch))
        .map(char::len_utf8)
        .sum::<usize>();
    let core_end = body.len().saturating_sub(trailing_len);

    let leading = &body[..leading_len];
    let core = &body[leading_len..core_end];
    let trailing = &body[core_end..];

    if let Some(query) = parse_file_tag_query(core) {
        push_text_chunk(chunks, leading.to_string());
        chunks.push(InjectChunk::FileTag(query.to_string()));
        push_text_chunk(chunks, format!("{}{}", trailing, trailing_ws));
        return;
    }

    push_text_chunk(chunks, segment.to_string());
}

fn parse_file_tag_query(core: &str) -> Option<&str> {
    if core.is_empty() {
        return None;
    }

    let query = core.strip_prefix('@').unwrap_or(core);
    if query.is_empty() || query.contains("://") || query.contains('@') {
        return None;
    }

    if !query.chars().all(is_file_tag_char) {
        return None;
    }

    if !has_known_file_extension(query) {
        return None;
    }

    Some(query)
}

fn is_file_tag_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/')
}

fn is_file_tag_leading_char(ch: char) -> bool {
    is_file_tag_char(ch) || ch == '@'
}

fn is_file_tag_trailing_char(ch: char) -> bool {
    is_file_tag_char(ch)
}

fn push_text_chunk(chunks: &mut Vec<InjectChunk>, text: String) {
    if text.is_empty() {
        return;
    }

    if let Some(InjectChunk::Text(existing)) = chunks.last_mut() {
        existing.push_str(&text);
    } else {
        chunks.push(InjectChunk::Text(text));
    }
}

fn has_known_file_extension(query: &str) -> bool {
    const KNOWN_FILE_EXTENSIONS: &[&str] = &[
        ".c", ".cc", ".cpp", ".css", ".go", ".h", ".hpp", ".html", ".java", ".js", ".json", ".jsx",
        ".md", ".mdx", ".py", ".rs", ".sql", ".swift", ".toml", ".ts", ".tsx", ".txt", ".xml",
        ".yaml", ".yml",
    ];

    let lower = query.to_ascii_lowercase();
    KNOWN_FILE_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(extension))
}

#[cfg(test)]
mod tests {
    use super::{split_inject_chunks, InjectChunk};

    #[test]
    fn splits_cursor_mentions_into_actionable_chunks() {
        let chunks = split_inject_chunks("Check @user.ts and @src/auth.tsx before shipping.");
        assert_eq!(chunks.len(), 5);
        assert!(matches!(&chunks[0], InjectChunk::Text(text) if text == "Check "));
        assert!(matches!(&chunks[1], InjectChunk::FileTag(tag) if tag == "user.ts"));
        assert!(matches!(&chunks[2], InjectChunk::Text(text) if text == " and "));
        assert!(matches!(&chunks[3], InjectChunk::FileTag(tag) if tag == "src/auth.tsx"));
        assert!(matches!(&chunks[4], InjectChunk::Text(text) if text == " before shipping."));
    }

    #[test]
    fn splits_bare_filenames_into_actionable_chunks() {
        let chunks = split_inject_chunks("Check users.ts and src/auth.tsx before shipping.");
        assert_eq!(chunks.len(), 5);
        assert!(matches!(&chunks[0], InjectChunk::Text(text) if text == "Check "));
        assert!(matches!(&chunks[1], InjectChunk::FileTag(tag) if tag == "users.ts"));
        assert!(matches!(&chunks[2], InjectChunk::Text(text) if text == " and "));
        assert!(matches!(&chunks[3], InjectChunk::FileTag(tag) if tag == "src/auth.tsx"));
        assert!(matches!(&chunks[4], InjectChunk::Text(text) if text == " before shipping."));
    }

    #[test]
    fn leaves_emails_alone() {
        let chunks = split_inject_chunks("Email us at hello@example.com please.");
        assert_eq!(chunks.len(), 1);
        assert!(
            matches!(&chunks[0], InjectChunk::Text(text) if text == "Email us at hello@example.com please.")
        );
    }
}
