#![allow(dead_code)]

pub mod deepgram;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Deepgram,
    Groq,
    OpenAi,
    Local,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptChunk {
    pub text: String,
    pub is_final: bool,
    pub confidence: f32,
}
