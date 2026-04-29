pub mod anthropic;
pub mod openrouter;

use crate::settings::DictationConfig;

const KNOWN_FILE_EXTENSIONS: &[&str] = &[
    "c", "cc", "cpp", "css", "go", "h", "hpp", "html", "java", "js", "json", "jsx", "md", "mdx",
    "py", "rs", "sql", "swift", "toml", "ts", "tsx", "txt", "xml", "yaml", "yml",
];

const FILE_SEPARATOR_WORDS: &[&str] = &[
    "dot",
    "period",
    "slash",
    "forwardslash",
    "backslash",
    "underscore",
    "dash",
    "hyphen",
];

const FILE_TAG_TRIGGER_WORDS: &[&str] = &["at", "tag", "tagged"];
const FILE_TAG_STOP_WORDS: &[&str] = &[
    "and", "as", "because", "but", "for", "from", "if", "in", "into", "is", "it", "of", "on", "or",
    "please", "so", "that", "the", "then", "to", "with",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupSurface {
    CodeEditor,
    EmailClient,
    Chat,
    Generic,
}

impl CleanupSurface {
    fn label(self) -> &'static str {
        match self {
            Self::CodeEditor => "a code editor or AI coding chat",
            Self::EmailClient => "an email client",
            Self::Chat => "a chat or messaging app",
            Self::Generic => "a general text field",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CleanupContext {
    pub source_app: Option<String>,
    pub surface: CleanupSurface,
    pub format_as_email: bool,
    pub format_as_bullets: bool,
    pub format_spoken_file_tags: bool,
}

impl CleanupContext {
    pub fn has_structural_formatting(&self) -> bool {
        self.format_as_email || self.format_as_bullets || self.format_spoken_file_tags
    }
}

pub fn infer_cleanup_context(transcript: &str, source_app: Option<&str>) -> CleanupContext {
    let surface = infer_surface(source_app);
    let format_as_email = looks_like_email(transcript, surface);
    let format_as_bullets = looks_like_bullet_list(transcript);
    let format_spoken_file_tags = looks_like_spoken_file_tag(transcript, surface);

    CleanupContext {
        source_app: source_app
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        surface,
        format_as_email,
        format_as_bullets,
        format_spoken_file_tags,
    }
}

pub fn should_run_cleanup(cfg: &DictationConfig, context: &CleanupContext) -> bool {
    let translation_requested = !(cfg.translate_to == "same" || cfg.translate_to.trim().is_empty());
    translation_requested
        || !cfg.custom_cleanup_prompt.trim().is_empty()
        || cfg.auto_cleanup != "none"
        || context.has_structural_formatting()
}

pub fn apply_best_effort_output_transforms(text: &str, context: &CleanupContext) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }

    let mut out = text.to_string();
    if context.format_spoken_file_tags {
        out = normalize_spoken_file_tags(&out);
    }
    out
}

/// System prompt shared by every LLM cleanup client. Auto Cleanup level,
/// dictionary words, and light target-app context drive the variant.
pub fn build_system_prompt(
    cfg: &DictationConfig,
    dictionary: &[String],
    context: &CleanupContext,
) -> String {
    let cleanup_directive = match cfg.auto_cleanup.as_str() {
        "none" => "Preserve the transcript as literally as possible. Do not rewrite for style unless the context-specific rules below require structural formatting.",
        "light" => "Apply only minimal cleanup — fix obvious typos and add basic punctuation. Preserve every word the speaker said.",
        "high" => "Apply aggressive cleanup. Drop all filler words, restructure run-on sentences for clarity, fix grammar, format lists when natural. Never invent new content.",
        _ => "Apply moderate cleanup. Drop filler words (um, uh, you know, like) unless meaningful. Fix grammar and punctuation. Preserve the speaker's intent and tone.",
    };
    let translation_directive = if cfg.translate_to == "same" || cfg.translate_to.trim().is_empty()
    {
        String::new()
    } else {
        format!(
            "\n- Translate the final output into {}.",
            cfg.translate_to.trim()
        )
    };

    let dictionary_section = if !dictionary.is_empty() {
        let limited: Vec<&String> = dictionary.iter().take(200).collect();
        format!(
            "\n\nWords/names/terms to spell carefully (use only when the speaker clearly said them — never insert):\n{}",
            limited
                .iter()
                .map(|w| format!("- {}", w))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        String::new()
    };
    let custom_prompt = if cfg.custom_cleanup_prompt.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n\nAdditional user instructions:\n{}",
            cfg.custom_cleanup_prompt.trim()
        )
    };
    let context_rules = render_context_rules(context);

    format!(
        r#"You are a dictation post-processor. You receive raw speech-to-text output and return clean text ready to be typed into an application.

Your job:
- {cleanup}
- Fix spelling, grammar, and punctuation errors.{translation}
- Preserve the speaker's intent, tone, and meaning exactly.
- Never insert names, terms, or content the speaker did not say.{context_rules}

Output rules:
- Return ONLY the cleaned transcript text — no preamble, no quotes around it, no explanation.
- Never output prefixes like "Here is the cleaned transcript:".
- If the transcription is empty or contains no meaningful speech, return exactly: EMPTY
- Do not change the meaning of what was said.{dictionary}{custom_prompt}

Example:
RAW_TRANSCRIPTION: hey um so i wanted to like follow up on the meating from yesterday i think we should definately move the dedline to next friday
Output: Hey, I wanted to follow up on the meeting from yesterday. I think we should definitely move the deadline to next Friday."#,
        cleanup = cleanup_directive,
        translation = translation_directive,
        context_rules = context_rules,
        dictionary = dictionary_section,
        custom_prompt = custom_prompt
    )
}

fn render_context_rules(context: &CleanupContext) -> String {
    let mut lines = Vec::new();

    if let Some(app) = &context.source_app {
        lines.push(format!(
            "- The active destination app appears to be {}. Treat the target surface as {}.",
            app,
            context.surface.label()
        ));
    } else if context.surface != CleanupSurface::Generic {
        lines.push(format!(
            "- Treat the target surface as {}.",
            context.surface.label()
        ));
    }

    if context.surface == CleanupSurface::CodeEditor {
        lines.push(
            "- Preserve code identifiers, file paths, punctuation, and casing when they already look intentional."
                .to_string(),
        );
    }

    if context.format_spoken_file_tags {
        lines.push(
            "- When the speaker uses explicit spoken file syntax such as \"at auth dot ts\" or \"src slash components slash button dot tsx\", normalize it into a literal @file reference like `@auth.ts` or `@src/components/button.tsx` when unambiguous. Convert spoken separators like \"dot\", \"slash\", \"underscore\", and \"dash\" only when they clearly describe a file path. Never invent filenames or turn email addresses into file tags.".to_string(),
        );
    }

    if context.format_as_email {
        lines.push(
            "- The transcript likely belongs to an email. Keep greetings and sign-offs on their own lines, preserve paragraph breaks, and do not invent a subject line or extra pleasantries."
                .to_string(),
        );
    }

    if context.format_as_bullets {
        lines.push(
            "- The transcript sounds like a spoken list. Format the final output as concise bullet points using `- `, one item per line. Remove spoken list markers like \"bullet point\", \"next bullet\", or ordinal scaffolding when they are only structural."
                .to_string(),
        );
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("\n\nContext-specific rules:\n{}", lines.join("\n"))
    }
}

fn infer_surface(source_app: Option<&str>) -> CleanupSurface {
    let lower = source_app.unwrap_or_default().trim().to_ascii_lowercase();
    if lower.is_empty() {
        return CleanupSurface::Generic;
    }

    if matches!(lower.as_str(), "cursor" | "windsurf" | "zed" | "xcode")
        || lower.contains("visual studio code")
        || lower == "code"
    {
        CleanupSurface::CodeEditor
    } else if lower.contains("mail")
        || lower.contains("outlook")
        || lower.contains("superhuman")
        || lower.contains("spark")
    {
        CleanupSurface::EmailClient
    } else if lower.contains("slack")
        || lower.contains("discord")
        || lower.contains("messages")
        || lower.contains("telegram")
        || lower.contains("teams")
    {
        CleanupSurface::Chat
    } else {
        CleanupSurface::Generic
    }
}

fn looks_like_email(transcript: &str, surface: CleanupSurface) -> bool {
    if surface == CleanupSurface::EmailClient {
        return true;
    }

    let lower = format!(" {} ", transcript.trim().to_ascii_lowercase());
    let has_greeting = [
        " hi ",
        " hello ",
        " hey ",
        " dear ",
        " good morning ",
        " good afternoon ",
        " good evening ",
    ]
    .iter()
    .any(|needle| lower.starts_with(needle) || lower.contains(&format!("\n{}", needle.trim())));
    let has_signoff = [
        " regards",
        " best",
        " sincerely",
        " thanks,",
        " thank you,",
        " many thanks",
        " talk soon",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    has_greeting && has_signoff
}

fn looks_like_bullet_list(transcript: &str) -> bool {
    let lower = format!(" {} ", transcript.trim().to_ascii_lowercase());
    if [
        " bullet point ",
        " bullet points ",
        " next bullet ",
        " next item ",
        " first bullet ",
        " second bullet ",
        " item one ",
        " item two ",
        " number one ",
        " number two ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return true;
    }

    let ordinal_hits = [
        " first ",
        " second ",
        " third ",
        " fourth ",
        " fifth ",
        " finally ",
        " lastly ",
    ]
    .iter()
    .filter(|needle| lower.contains(**needle))
    .count();
    ordinal_hits >= 2
}

fn looks_like_spoken_file_tag(transcript: &str, surface: CleanupSurface) -> bool {
    let lower = format!(" {} ", transcript.trim().to_ascii_lowercase());
    let mentions_extension = KNOWN_FILE_EXTENSIONS
        .iter()
        .any(|ext| lower.contains(&format!(" dot {} ", ext)));
    if !mentions_extension {
        return false;
    }

    surface == CleanupSurface::CodeEditor
        || FILE_TAG_TRIGGER_WORDS
            .iter()
            .any(|needle| lower.contains(&format!(" {} ", needle)))
        || FILE_SEPARATOR_WORDS
            .iter()
            .any(|needle| lower.contains(&format!(" {} ", needle)))
}

fn normalize_spoken_file_tags(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return text.to_string();
    }

    let mut out = Vec::with_capacity(tokens.len());
    let mut index = 0usize;
    while index < tokens.len() {
        let normalized = normalize_token(tokens[index]);
        if FILE_TAG_TRIGGER_WORDS.contains(&normalized.as_str()) {
            if let Some((tag, next_index)) = parse_spoken_file_tag(&tokens, index + 1) {
                out.push(tag);
                index = next_index;
                continue;
            }
        }

        out.push(tokens[index].to_string());
        index += 1;
    }

    out.join(" ")
}

fn parse_spoken_file_tag(tokens: &[&str], start: usize) -> Option<(String, usize)> {
    if start >= tokens.len() {
        return None;
    }

    let mut path = String::new();
    let mut index = start;
    let mut saw_separator = false;
    let mut saw_extension = false;
    let mut trailing = String::new();

    while index < tokens.len() && index < start + 16 {
        let token = tokens[index];
        let core = trim_outer_non_path_chars(token);
        let normalized = normalize_token(token);
        if normalized.is_empty() {
            break;
        }

        if !path.is_empty() && FILE_TAG_STOP_WORDS.contains(&normalized.as_str()) {
            break;
        }

        let segment = match normalized.as_str() {
            "dot" | "period" => {
                saw_separator = true;
                "."
            }
            "slash" | "forwardslash" | "backslash" => {
                saw_separator = true;
                "/"
            }
            "underscore" => {
                saw_separator = true;
                "_"
            }
            "dash" | "hyphen" => {
                saw_separator = true;
                "-"
            }
            _ => {
                if core.is_empty()
                    || !should_accept_file_word(tokens, index, start, &path, &normalized)
                {
                    break;
                }
                if KNOWN_FILE_EXTENSIONS.contains(&normalized.as_str()) && path.ends_with('.') {
                    saw_extension = true;
                }
                core
            }
        };

        path.push_str(segment);
        trailing = trailing_punctuation(token);
        index += 1;
    }

    if !saw_separator || !saw_extension || !path_has_known_extension(&path) {
        return None;
    }

    Some((format!("@{}{}", path, trailing), index))
}

fn should_accept_file_word(
    tokens: &[&str],
    index: usize,
    start: usize,
    path: &str,
    normalized: &str,
) -> bool {
    if path.ends_with(['.', '/', '_', '-']) {
        return true;
    }

    if start == index {
        return next_token_is_separator_or_extension(tokens, index, normalized);
    }

    next_token_is_separator_or_extension(tokens, index, normalized)
}

fn next_token_is_separator_or_extension(tokens: &[&str], index: usize, normalized: &str) -> bool {
    let next = tokens
        .get(index + 1)
        .map(|token| normalize_token(token))
        .unwrap_or_default();
    FILE_SEPARATOR_WORDS.contains(&next.as_str()) || KNOWN_FILE_EXTENSIONS.contains(&normalized)
}

fn path_has_known_extension(path: &str) -> bool {
    KNOWN_FILE_EXTENSIONS
        .iter()
        .any(|ext| path.to_ascii_lowercase().ends_with(&format!(".{}", ext)))
}

fn normalize_token(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '@')
        .trim_start_matches('@')
        .to_ascii_lowercase()
}

fn trim_outer_non_path_chars(token: &str) -> &str {
    token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.')
}

fn trailing_punctuation(token: &str) -> String {
    token
        .chars()
        .rev()
        .take_while(|c| matches!(c, '.' | ',' | '!' | '?' | ';' | ':'))
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_best_effort_output_transforms, infer_cleanup_context, CleanupContext, CleanupSurface,
    };

    #[test]
    fn detects_email_context_from_content() {
        let context = infer_cleanup_context(
            "Hi Mei, thanks again for the quick turnaround. Regards, Usman",
            None,
        );
        assert!(context.format_as_email);
    }

    #[test]
    fn detects_spoken_bullet_lists() {
        let context = infer_cleanup_context(
            "First fix auth. Second update tests. Third push the release.",
            None,
        );
        assert!(context.format_as_bullets);
    }

    #[test]
    fn normalizes_spoken_file_tags() {
        let context = CleanupContext {
            source_app: Some("Cursor".to_string()),
            surface: CleanupSurface::CodeEditor,
            format_as_email: false,
            format_as_bullets: false,
            format_spoken_file_tags: true,
        };

        assert_eq!(
            apply_best_effort_output_transforms(
                "Refactor at auth slash use slash session dot tsx please.",
                &context
            ),
            "Refactor @auth/use/session.tsx please."
        );
    }
}
