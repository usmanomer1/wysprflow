// Deepgram Nova-3 streaming client.
// Spawns a sender task that pumps audio bytes into a WebSocket and a receiver task
// that parses JSON Results into TranscriptChunks emitted on a tokio mpsc channel.

use anyhow::{bail, Context, Result};
use futures_util::{SinkExt, StreamExt};
use http::header::AUTHORIZATION;
use http::HeaderValue;
use serde::Deserialize;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

use super::TranscriptChunk;

pub struct DeepgramClient {
    api_key: String,
}

impl DeepgramClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub async fn validate(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.deepgram.com/v1/projects")
            .header("Authorization", format!("Token {}", self.api_key))
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Deepgram /v1/projects returned {}: {}", status, body)
        }
    }
}

pub struct DeepgramSession {
    /// Send raw PCM16 LE bytes here to forward them to Deepgram.
    pub audio_tx: UnboundedSender<Vec<u8>>,
    /// Receive transcript chunks from Deepgram here.
    pub transcripts: UnboundedReceiver<TranscriptChunk>,
    /// Signal CloseStream + close the WebSocket. Must be called after audio is done.
    pub close: tokio::sync::oneshot::Sender<()>,
}

pub struct DeepgramConnectArgs<'a> {
    pub api_key: &'a str,
    pub sample_rate: u32,
    pub language: &'a str,
    pub model: &'a str,
    pub interim_results: bool,
    pub endpointing_ms: u32,
    pub smart_format: bool,
}

impl<'a> Default for DeepgramConnectArgs<'a> {
    fn default() -> Self {
        Self {
            api_key: "",
            sample_rate: 16_000,
            language: "en",
            model: "nova-3",
            interim_results: true,
            endpointing_ms: 300,
            smart_format: true,
        }
    }
}

pub async fn connect(args: DeepgramConnectArgs<'_>) -> Result<DeepgramSession> {
    let url = build_url(&args);
    info!("deepgram: connecting to {}", url);

    let mut request = url.as_str().into_client_request().context("ws request")?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Token {}", args.api_key))
            .context("invalid api key for header")?,
    );

    let (ws, response) = connect_async(request)
        .await
        .context("deepgram connect_async")?;
    debug!("deepgram: HTTP upgrade status = {}", response.status());

    let (audio_tx, audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (transcript_tx, transcripts) = mpsc::unbounded_channel::<TranscriptChunk>();
    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

    tauri::async_runtime::spawn(run_session(ws, audio_rx, transcript_tx, close_rx));

    Ok(DeepgramSession {
        audio_tx,
        transcripts,
        close: close_tx,
    })
}

fn build_url(args: &DeepgramConnectArgs<'_>) -> String {
    let mut url = String::from("wss://api.deepgram.com/v1/listen?");
    let _ = std::fmt::Write::write_fmt(
        &mut url,
        format_args!(
            "model={}&encoding=linear16&sample_rate={}&channels=1&interim_results={}&punctuate=true&smart_format={}&endpointing={}",
            args.model, args.sample_rate, args.interim_results, args.smart_format, args.endpointing_ms
        ),
    );
    if !args.language.is_empty() && args.language != "auto" {
        url.push_str(&format!("&language={}", args.language));
    }
    url
}

async fn run_session(
    ws: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    mut audio_rx: UnboundedReceiver<Vec<u8>>,
    transcript_tx: UnboundedSender<TranscriptChunk>,
    mut close_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let (mut sink, mut stream) = ws.split();

    // Receiver side: parse incoming Deepgram JSON messages → transcript chunks.
    let recv_task = tauri::async_runtime::spawn(async move {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    if let Some(chunk) = parse_result(&t) {
                        let _ = transcript_tx.send(chunk);
                    }
                }
                Ok(Message::Binary(_)) => {}
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(e) => {
                    error!("deepgram ws recv error: {}", e);
                    break;
                }
            }
        }
    });

    // Sender side: pump audio frames + listen for close signal.
    loop {
        tokio::select! {
            biased;
            _ = &mut close_rx => {
                // Send the CloseStream control message so Deepgram flushes the final transcript,
                // then close the socket cleanly.
                let _ = sink.send(Message::Text(r#"{"type":"CloseStream"}"#.into())).await;
                let _ = sink.close().await;
                break;
            }
            frame = audio_rx.recv() => {
                match frame {
                    Some(bytes) => {
                        if let Err(e) = sink.send(Message::Binary(bytes.into())).await {
                            warn!("deepgram ws send error: {}", e);
                            break;
                        }
                    }
                    None => {
                        // Audio source dropped — finalize the session.
                        let _ = sink.send(Message::Text(r#"{"type":"CloseStream"}"#.into())).await;
                        let _ = sink.close().await;
                        break;
                    }
                }
            }
        }
    }

    // Wait for the receiver task to finish draining the final transcript chunks.
    let _ = recv_task.await;
}

#[derive(Debug, Deserialize)]
struct DgEnvelope {
    #[serde(rename = "type")]
    kind: Option<String>,
    is_final: Option<bool>,
    speech_final: Option<bool>,
    channel: Option<DgChannel>,
}

#[derive(Debug, Deserialize)]
struct DgChannel {
    alternatives: Vec<DgAlternative>,
}

#[derive(Debug, Deserialize)]
struct DgAlternative {
    transcript: Option<String>,
    confidence: Option<f32>,
}

fn parse_result(payload: &str) -> Option<TranscriptChunk> {
    let env: DgEnvelope = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return None,
    };
    if env.kind.as_deref() != Some("Results") {
        return None;
    }
    let channel = env.channel?;
    let first = channel.alternatives.into_iter().next()?;
    let text = first.transcript?;
    if text.trim().is_empty() {
        return None;
    }
    Some(TranscriptChunk {
        text,
        is_final: env.is_final.unwrap_or(false) || env.speech_final.unwrap_or(false),
        confidence: first.confidence.unwrap_or(0.0),
    })
}
