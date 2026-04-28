// cpal-driven microphone capture.
//
// `cpal::Stream` is `!Send` on most platforms, so we can't hold it across an `.await`
// boundary in our pipeline task. Instead, we spawn a dedicated OS thread that owns
// the stream and pushes audio frames into a tokio mpsc channel. The pipeline holds a
// `CaptureHandle` (which is `Send`) and dropping it shuts the thread down.

use std::sync::mpsc as stdmpsc;
use std::sync::Arc;
use std::thread;

use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub struct CaptureHandle {
    /// Dropping the sender (or sending on it) tells the audio thread to stop the stream.
    shutdown_tx: Option<stdmpsc::Sender<()>>,
    level: Arc<Mutex<f32>>,
    pub sample_rate: u32,
    pub device_name: String,
}

impl CaptureHandle {
    pub fn level(&self) -> f32 {
        *self.level.lock()
    }

    pub fn level_arc(&self) -> Arc<Mutex<f32>> {
        self.level.clone()
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Start mic capture. The `audio_tx` channel receives mono f32 PCM frames at the
/// device's native sample rate (also exposed on the returned handle).
pub fn start(
    device_uid: Option<String>,
    audio_tx: UnboundedSender<Vec<f32>>,
) -> Result<CaptureHandle> {
    let level = Arc::new(Mutex::new(0.0_f32));
    let (rate_tx, rate_rx) = stdmpsc::channel::<Result<(u32, String), String>>();
    let (shutdown_tx, shutdown_rx) = stdmpsc::channel::<()>();

    let level_thread = level.clone();
    thread::Builder::new()
        .name("wysprflow-audio".into())
        .spawn(move || {
            run_audio_thread(device_uid, audio_tx, level_thread, rate_tx, shutdown_rx);
        })
        .context("spawn audio thread")?;

    let (sample_rate, device_name) = match rate_rx.recv() {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => bail!("audio init: {}", e),
        Err(_) => bail!("audio thread terminated before init"),
    };

    Ok(CaptureHandle {
        shutdown_tx: Some(shutdown_tx),
        level,
        sample_rate,
        device_name,
    })
}

pub fn list_input_devices() -> Result<Vec<AudioInputDevice>> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());

    let mut devices = Vec::new();
    for device in host.input_devices().context("input_devices")? {
        let name = device.name().unwrap_or_else(|_| "(unknown input)".into());
        devices.push(AudioInputDevice {
            id: name.clone(),
            name: name.clone(),
            is_default: default_name.as_deref() == Some(name.as_str()),
        });
    }

    devices.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(devices)
}

fn run_audio_thread(
    device_uid: Option<String>,
    audio_tx: UnboundedSender<Vec<f32>>,
    level: Arc<Mutex<f32>>,
    rate_tx: stdmpsc::Sender<Result<(u32, String), String>>,
    shutdown_rx: stdmpsc::Receiver<()>,
) {
    let host = cpal::default_host();
    let device = match pick_device(&host, device_uid.as_deref()) {
        Some(d) => d,
        None => {
            let _ = rate_tx.send(Err("no input device".into()));
            return;
        }
    };
    let device_name = device.name().unwrap_or_else(|_| "(unknown)".into());
    let cfg = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = rate_tx.send(Err(format!("default_input_config: {}", e)));
            return;
        }
    };
    let sample_rate = cfg.sample_rate().0;
    let channels = cfg.channels() as usize;
    info!(
        "audio: device={}, rate={}Hz, channels={}, format={:?}",
        device_name,
        sample_rate,
        channels,
        cfg.sample_format()
    );

    let stream_config = cfg.config();
    let level_cb = level.clone();
    let tx_cb = audio_tx.clone();

    let stream_result = match cfg.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &_| process_frame(data, channels, &tx_cb, &level_cb),
            |e| error!("cpal stream error: {}", e),
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _: &_| {
                let f: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                process_frame(&f, channels, &tx_cb, &level_cb);
            },
            |e| error!("cpal stream error: {}", e),
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _: &_| {
                let f: Vec<f32> = data
                    .iter()
                    .map(|&s| (s as f32 - 32768.0) / 32768.0)
                    .collect();
                process_frame(&f, channels, &tx_cb, &level_cb);
            },
            |e| error!("cpal stream error: {}", e),
            None,
        ),
        other => {
            let _ = rate_tx.send(Err(format!("unsupported sample format: {:?}", other)));
            return;
        }
    };

    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            let _ = rate_tx.send(Err(format!("build_input_stream: {}", e)));
            return;
        }
    };

    if let Err(e) = stream.play() {
        let _ = rate_tx.send(Err(format!("stream.play: {}", e)));
        return;
    }

    let _ = rate_tx.send(Ok((sample_rate, device_name)));

    // Block until the handle is dropped or stop is signaled.
    let _ = shutdown_rx.recv();
    *level.lock() = 0.0;
    drop(stream);
}

fn process_frame(
    data: &[f32],
    channels: usize,
    tx: &UnboundedSender<Vec<f32>>,
    level: &Arc<Mutex<f32>>,
) {
    let mono: Vec<f32> = if channels <= 1 {
        data.to_vec()
    } else {
        data.chunks_exact(channels)
            .map(|c| c.iter().copied().sum::<f32>() / channels as f32)
            .collect()
    };
    if mono.is_empty() {
        return;
    }

    let sum_sq: f32 = mono.iter().map(|s| s * s).sum();
    let rms = (sum_sq / mono.len() as f32).sqrt();
    *level.lock() = (rms * 1.5).min(1.0);

    // Diagnostic: every ~100th frame, log RMS so we can tell if the mic is silent
    // (rms ≈ 0.0) vs picking up speech (rms ≈ 0.05–0.4).
    static FRAME_LOG_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = FRAME_LOG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n % 100 == 0 {
        tracing::debug!("audio: frame {} samples={} rms={:.4}", n, mono.len(), rms);
    }

    let _ = tx.send(mono);
}

fn pick_device(host: &cpal::Host, uid: Option<&str>) -> Option<cpal::Device> {
    if let Some(name) = uid {
        if name != "default" && !name.is_empty() {
            if let Ok(devices) = host.input_devices() {
                for d in devices {
                    if d.name().ok().as_deref() == Some(name) {
                        return Some(d);
                    }
                }
            }
        }
    }
    host.default_input_device()
}

/// Float PCM (-1..1) to little-endian int16 PCM bytes for Deepgram's `linear16`.
pub fn f32_to_pcm16_le(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}
