// OpenRouter client. Uses the OpenAI-compatible /v1/chat/completions endpoint to
// route to any OpenRouter-supported model. Default for the cleanup pass is
// `anthropic/claude-haiku-4.5` so users get the same Haiku we use direct.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

use super::{
    apply_best_effort_output_transforms, build_system_prompt, should_run_cleanup, CleanupContext,
};
use crate::settings::DictationConfig;

pub struct OpenRouterClient {
    api_key: String,
    base: String,
}

impl OpenRouterClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base: "https://openrouter.ai/api/v1".into(),
        }
    }

    pub async fn validate(&self) -> Result<()> {
        // /auth/key returns 200 + JSON with rate-limit info when the key is valid.
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/auth/key", self.base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("OpenRouter /auth/key returned {}: {}", status, body)
        }
    }

    pub async fn cleanup(
        &self,
        transcript: &str,
        cfg: &DictationConfig,
        dictionary: &[String],
        context: &CleanupContext,
    ) -> Result<String> {
        if !should_run_cleanup(cfg, context) {
            return Ok(apply_best_effort_output_transforms(transcript, context));
        }

        let model = pick_model(&cfg.llm_model);
        let system = build_system_prompt(cfg, dictionary, context);

        let body = json!({
            "model": model,
            "max_tokens": 256,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": format!("RAW_TRANSCRIPTION: {}", transcript) }
            ],
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/chat/completions", self.base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            // OpenRouter's analytics headers — optional but recommended.
            .header("HTTP-Referer", "https://github.com/usmanumer/wysprflow")
            .header("X-Title", "wysprflow")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("openrouter POST /chat/completions")?;

        info!(
            "openrouter: response status={} model={}",
            resp.status(),
            model
        );

        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            bail!("openrouter returned {}: {}", status, txt);
        }

        let payload: OpenRouterResponse = resp.json().await.context("openrouter decode")?;
        let text = payload
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        let trimmed = text.trim();
        if trimmed.eq_ignore_ascii_case("EMPTY") {
            return Ok(String::new());
        }
        Ok(apply_best_effort_output_transforms(trimmed, context))
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterMessage {
    content: Option<String>,
}

/// If the user kept the Anthropic-direct default model id, swap to the OpenRouter
/// equivalent. Otherwise pass through whatever they configured.
fn pick_model(configured: &str) -> &str {
    if configured.is_empty() || configured == "claude-haiku-4-5" {
        "anthropic/claude-haiku-4.5"
    } else {
        configured
    }
}
