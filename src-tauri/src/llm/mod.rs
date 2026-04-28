pub mod anthropic;
pub mod openrouter;

use crate::settings::DictationConfig;

/// System prompt shared by every LLM cleanup client. Auto Cleanup level + dictionary
/// words drive the variant.
pub fn build_system_prompt(cfg: &DictationConfig, dictionary: &[String]) -> String {
    let cleanup_directive = match cfg.auto_cleanup.as_str() {
        "none" => "Preserve the transcript as literally as possible. Do not rewrite for style. Only fix obvious speech-recognition artifacts that block readability.",
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

    format!(
        r#"You are a dictation post-processor. You receive raw speech-to-text output and return clean text ready to be typed into an application.

Your job:
- {cleanup}
- Fix spelling, grammar, and punctuation errors.{translation}
- Preserve the speaker's intent, tone, and meaning exactly.
- Never insert names, terms, or content the speaker did not say.

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
        dictionary = dictionary_section,
        custom_prompt = custom_prompt
    )
}
