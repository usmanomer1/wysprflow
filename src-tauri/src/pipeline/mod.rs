// Dictation pipeline orchestrator. State machine:
//
//   Idle ── start() ──► Active { capture → Deepgram → cleanup → snippet → inject } ── stop() ──► Idle
//
// Phase 4: Pulls dictionary words into the cleanup prompt, matches the cleaned
// transcript against snippet triggers, and records every session into history.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::audio::capture::{self, f32_to_pcm16_le};
use crate::ax;
use crate::db::Db;
use crate::dictionary;
use crate::history::{self, NewEntry};
use crate::hud::{self, HudState};
use crate::llm::anthropic::AnthropicClient;
use crate::llm::openrouter::OpenRouterClient;
use crate::settings::{self, keychain, DictationConfig};
use crate::snippets;
use crate::stt::deepgram::{self, DeepgramConnectArgs};

#[derive(Clone)]
pub struct Pipeline {
    inner: Arc<Mutex<Inner>>,
    app: AppHandle,
}

struct Inner {
    state: State,
}

enum State {
    Idle,
    Active {
        session_id: u64,
        cancel: oneshot::Sender<()>,
        join: JoinHandle<()>,
    },
    Stopping {
        session_id: u64,
    },
}

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

impl Pipeline {
    pub fn new(app: AppHandle) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner { state: State::Idle })),
            app,
        }
    }

    pub fn start(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        if !matches!(inner.state, State::Idle) {
            return Ok(());
        }

        let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let app = self.app.clone();
        let join = tauri::async_runtime::spawn(async move {
            run_session(app, cancel_rx, session_id).await;
        });
        inner.state = State::Active {
            session_id,
            cancel: cancel_tx,
            join,
        };
        info!("pipeline[{session_id}]: started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let join_handle = {
            let mut inner = self.inner.lock();
            match std::mem::replace(&mut inner.state, State::Idle) {
                State::Active {
                    session_id,
                    cancel,
                    join,
                } => {
                    let _ = cancel.send(());
                    inner.state = State::Stopping { session_id };
                    Some((session_id, join))
                }
                State::Idle => {
                    inner.state = State::Idle;
                    None
                }
                State::Stopping { session_id } => {
                    inner.state = State::Stopping { session_id };
                    None
                }
            }
        };
        if let Some((session_id, join)) = join_handle {
            info!("pipeline[{session_id}]: stop requested");
            let _ = join.await;
            let mut inner = self.inner.lock();
            if matches!(inner.state, State::Stopping { session_id: current } if current == session_id)
            {
                inner.state = State::Idle;
            }
            info!("pipeline[{session_id}]: fully stopped");
        }
        Ok(())
    }
}

async fn run_session(app: AppHandle, mut cancel: oneshot::Receiver<()>, session_id: u64) {
    let cfg = settings::get();
    let started_at = Instant::now();

    let dg_key = match keychain::get("deepgram") {
        Ok(Some(k)) => k,
        _ => {
            warn!("pipeline[{session_id}]: Deepgram key missing");
            let _ = hud::emit_state(&app, HudState::error("Add Deepgram key"));
            let _ = hud::show(&app);
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = hud::hide(&app);
            return;
        }
    };

    let _ = hud::show(&app);
    let _ = hud::emit_state(&app, HudState::initializing());

    // ---- Audio capture --------------------------------------------------------
    let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
    let device_uid = match cfg.microphone_device.as_str() {
        "" | "default" => None,
        other => Some(other.to_string()),
    };
    let capture = match capture::start(device_uid, audio_tx) {
        Ok(c) => c,
        Err(e) => {
            error!("pipeline[{session_id}]: audio start failed: {}", e);
            let _ = hud::emit_state(&app, HudState::error("Mic unavailable"));
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = hud::hide(&app);
            record_error(&app, "Mic unavailable", started_at);
            return;
        }
    };
    if cfg.play_sounds {
        play_feedback_sound("/System/Library/Sounds/Pop.aiff");
    }
    let sample_rate = capture.sample_rate;
    let level_handle = capture.level_arc();
    info!("pipeline[{session_id}]: capture rate = {}Hz", sample_rate);

    // ---- Deepgram session -----------------------------------------------------
    let dg_session = match deepgram::connect(DeepgramConnectArgs {
        api_key: &dg_key,
        sample_rate,
        language: if cfg.language == "auto" {
            ""
        } else {
            &cfg.language
        },
        model: "nova-3",
        interim_results: true,
        endpointing_ms: 300,
        smart_format: true,
    })
    .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("pipeline[{session_id}]: deepgram connect failed: {}", e);
            let _ = hud::emit_state(&app, HudState::error("Deepgram error"));
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = hud::hide(&app);
            drop(capture);
            record_error(&app, &format!("Deepgram: {}", e), started_at);
            return;
        }
    };
    let dg_audio_tx = dg_session.audio_tx.clone();
    let dg_close = dg_session.close;
    let mut dg_transcripts = dg_session.transcripts;

    // ---- Audio → Deepgram pump + HUD level updates ---------------------------
    let app_for_audio = app.clone();
    let mut audio_pump = tauri::async_runtime::spawn(async move {
        let mut last_emit = Instant::now();
        let mut frames = 0u64;
        let mut bytes = 0u64;
        while let Some(frame) = audio_rx.recv().await {
            let pcm = f32_to_pcm16_le(&frame);
            frames += 1;
            bytes += pcm.len() as u64;
            if dg_audio_tx.send(pcm).is_err() {
                break;
            }
            if last_emit.elapsed() >= Duration::from_millis(33) {
                let lvl = *level_handle.lock();
                let _ = hud::emit_state(&app_for_audio, HudState::listening(lvl));
                last_emit = Instant::now();
            }
        }
        tracing::info!(
            "audio_pump: ended after {} frames / {} bytes pushed to Deepgram",
            frames,
            bytes
        );
    });

    // ---- Deepgram → transcript buffer + UI events ----------------------------
    let app_for_transcripts = app.clone();
    let transcript_collector = tauri::async_runtime::spawn(async move {
        let mut final_text = String::new();
        let mut latest_partial = String::new();
        while let Some(chunk) = dg_transcripts.recv().await {
            let _ = app_for_transcripts.emit("transcript", &chunk);
            let trimmed = chunk.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            debug!(
                "pipeline[{session_id}]: transcript chunk final={} text={:?}",
                chunk.is_final, trimmed
            );
            if chunk.is_final {
                append_segment(&mut final_text, trimmed);
                latest_partial.clear();
            } else {
                latest_partial = trimmed.to_string();
            }
        }
        if final_text.is_empty() {
            latest_partial
        } else {
            if !latest_partial.is_empty() && !final_text.ends_with(&latest_partial) {
                append_segment(&mut final_text, &latest_partial);
            }
            final_text
        }
    });

    // ---- Wait for hotkey release ---------------------------------------------
    let _ = (&mut cancel).await;
    let _ = hud::emit_state(&app, HudState::processing_with_message("Transcribing"));

    // ---- Tear-down -----------------------------------------------------------
    drop(capture);
    match tokio::time::timeout(Duration::from_secs(1), &mut audio_pump).await {
        Ok(Ok(())) => {
            let _ = dg_close.send(());
        }
        Ok(Err(e)) => {
            warn!("pipeline[{session_id}]: audio_pump join failed: {}", e);
            let _ = dg_close.send(());
        }
        Err(_) => {
            warn!("pipeline[{session_id}]: audio_pump shutdown timed out; forcing Deepgram close");
            let _ = dg_close.send(());
            audio_pump.abort();
            let _ = audio_pump.await;
        }
    }

    let raw_transcript = transcript_collector.await.unwrap_or_default();
    let raw_trimmed = raw_transcript.trim().to_string();
    info!("pipeline[{session_id}]: raw transcript = {:?}", raw_trimmed);

    if raw_trimmed.is_empty() {
        let _ = hud::emit_state(&app, HudState::idle());
        let _ = hud::hide(&app);
        return;
    }

    // ---- LLM cleanup pass with dictionary -----------------------------------
    let dictionary_words = app
        .try_state::<Db>()
        .and_then(|db| dictionary::list_words(&db).ok())
        .unwrap_or_default();

    let _ = hud::emit_state(&app, HudState::processing_with_message("Cleaning"));
    info!("pipeline[{session_id}]: cleanup starting");
    let cleaned = match run_cleanup(&cfg, &raw_trimmed, &dictionary_words).await {
        Ok(t) => t,
        Err(e) => {
            warn!(
                "pipeline[{session_id}]: cleanup failed, falling back to raw transcript: {}",
                e
            );
            raw_trimmed.clone()
        }
    };
    info!("pipeline[{session_id}]: cleanup finished");

    if cleaned.is_empty() {
        let _ = hud::emit_state(&app, HudState::idle());
        let _ = hud::hide(&app);
        record_session(&app, &raw_trimmed, "", started_at, None);
        return;
    }

    // ---- Snippet expansion ---------------------------------------------------
    let final_text = if cfg.snippets_enabled {
        if let Some(db) = app.try_state::<Db>() {
            match snippets::match_trigger(&db, &cleaned) {
                Ok(Some(expansion)) => expansion,
                _ => cleaned.clone(),
            }
        } else {
            cleaned.clone()
        }
    } else {
        cleaned.clone()
    };

    let _ = app.emit("transcript-cleaned", &final_text);

    // ---- Inject ---------------------------------------------------------------
    let _ = hud::emit_state(&app, HudState::processing_with_message("Pasting"));
    info!("pipeline[{session_id}]: inject starting");
    let inject_error = if let Err(e) = ax::inject(&app, &final_text, cfg.preserve_clipboard).await {
        error!("pipeline[{session_id}]: inject failed: {}", e);
        let _ = hud::emit_state(&app, HudState::error("Couldn't paste"));
        tokio::time::sleep(Duration::from_secs(1)).await;
        Some(format!("Inject: {}", e))
    } else {
        info!("pipeline[{session_id}]: inject finished");
        None
    };
    if cfg.play_sounds {
        play_feedback_sound("/System/Library/Sounds/Tink.aiff");
    }

    // ---- Record into history --------------------------------------------------
    record_session(&app, &raw_trimmed, &final_text, started_at, inject_error);

    let _ = hud::emit_state(&app, HudState::idle());
    let _ = hud::hide(&app);
}

async fn run_cleanup(
    cfg: &DictationConfig,
    transcript: &str,
    dictionary: &[String],
) -> Result<String> {
    let cleanup_started = Instant::now();
    match cfg.llm_provider.as_str() {
        "anthropic" => {
            info!(
                "cleanup: provider=anthropic model={} chars={}",
                if cfg.llm_model.is_empty() {
                    "claude-haiku-4-5"
                } else {
                    cfg.llm_model.as_str()
                },
                transcript.len()
            );
            let key = keychain::get("anthropic")?
                .ok_or_else(|| anyhow::anyhow!("no Anthropic key in Keychain"))?;
            let out = AnthropicClient::new(key)
                .cleanup(transcript, cfg, dictionary)
                .await?;
            info!(
                "cleanup: provider=anthropic done in {}ms",
                cleanup_started.elapsed().as_millis()
            );
            Ok(out)
        }
        "openrouter" => {
            info!(
                "cleanup: provider=openrouter model={} chars={}",
                if cfg.llm_model.is_empty() || cfg.llm_model == "claude-haiku-4-5" {
                    "anthropic/claude-haiku-4.5"
                } else {
                    cfg.llm_model.as_str()
                },
                transcript.len()
            );
            let key = keychain::get("openrouter")?
                .ok_or_else(|| anyhow::anyhow!("no OpenRouter key in Keychain"))?;
            let out = OpenRouterClient::new(key)
                .cleanup(transcript, cfg, dictionary)
                .await?;
            info!(
                "cleanup: provider=openrouter done in {}ms",
                cleanup_started.elapsed().as_millis()
            );
            Ok(out)
        }
        // "off" or anything else — skip cleanup, return transcript verbatim.
        _ => {
            info!("cleanup: provider=off chars={}", transcript.len());
            Ok(transcript.to_string())
        }
    }
}

fn record_session(
    app: &AppHandle,
    raw: &str,
    cleaned: &str,
    started_at: Instant,
    error: Option<String>,
) {
    let Some(db) = app.try_state::<Db>() else {
        return;
    };
    let duration_ms = started_at.elapsed().as_millis() as i64;
    let entry = NewEntry {
        raw: raw.to_string(),
        cleaned: cleaned.to_string(),
        source_app: None,
        duration_ms: Some(duration_ms),
        error,
    };
    if let Err(e) = history::record(&db, entry) {
        warn!("history record failed: {}", e);
    }
}

fn record_error(app: &AppHandle, message: &str, started_at: Instant) {
    record_session(app, "", "", started_at, Some(message.to_string()));
}

fn append_segment(buf: &mut String, segment: &str) {
    if segment.is_empty() {
        return;
    }
    if !buf.is_empty() && !buf.ends_with(' ') {
        buf.push(' ');
    }
    buf.push_str(segment);
}

#[cfg(target_os = "macos")]
fn play_feedback_sound(path: &str) {
    let sound = path.to_string();
    std::thread::spawn(move || {
        let _ = std::process::Command::new("afplay").arg(sound).spawn();
    });
}

#[cfg(not(target_os = "macos"))]
fn play_feedback_sound(_path: &str) {}
