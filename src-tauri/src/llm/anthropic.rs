// Anthropic Claude API client (direct). Defaults to Haiku 4.5.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

use super::{
    apply_best_effort_output_transforms, build_system_prompt, should_run_cleanup, CleanupContext,
};
use crate::settings::DictationConfig;

pub struct AnthropicClient {
    api_key: String,
    base: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base: "https://api.anthropic.com".into(),
        }
    }

    pub async fn validate(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/v1/messages", self.base))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-haiku-4-5",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "."}],
            }))
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Anthropic /v1/messages returned {}: {}", status, body)
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

        let model = if cfg.llm_model.is_empty() {
            "claude-haiku-4-5"
        } else {
            cfg.llm_model.as_str()
        };
        let system = build_system_prompt(cfg, dictionary, context);
        let body = json!({
            "model": model,
            "max_tokens": 256,
            "system": system,
            "messages": [
                { "role": "user", "content": format!("RAW_TRANSCRIPTION: {}", transcript) }
            ],
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/v1/messages", self.base))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("anthropic POST /v1/messages")?;

        info!(
            "anthropic: response status={} model={}",
            resp.status(),
            model
        );

        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            bail!("anthropic returned {}: {}", status, txt);
        }

        let payload: AnthropicResponse = resp.json().await.context("anthropic decode")?;
        let text = payload
            .content
            .into_iter()
            .filter_map(|c| match c.kind.as_deref() {
                Some("text") => c.text,
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let trimmed = text.trim();
        if trimmed.eq_ignore_ascii_case("EMPTY") {
            return Ok(String::new());
        }
        Ok(apply_best_effort_output_transforms(trimmed, context))
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    kind: Option<String>,
    text: Option<String>,
}
