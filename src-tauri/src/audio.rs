use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct AudioRecorderState {
    pub stream: Mutex<Option<Stream>>,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: Mutex<u32>,
}

impl Default for AudioRecorderState {
    fn default() -> Self {
        Self {
            stream: Mutex::new(None),
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Mutex::new(16000),
        }
    }
}

pub async fn ensure_model_downloaded(app_data_dir: &Path) -> Result<PathBuf> {
    let model_dir = app_data_dir.join("models");
    if !model_dir.exists() {
        tokio::fs::create_dir_all(&model_dir).await?;
    }

    let model_path = model_dir.join("ggml-base.en.bin");
    if model_path.exists() {
        return Ok(model_path);
    }

    tracing::info!("Downloading whisper model base.en (140MB)...");
    let url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
    
    let response = reqwest::get(url).await?.error_for_status()?;
    let bytes = response.bytes().await?;
    
    tokio::fs::write(&model_path, bytes).await?;
    tracing::info!("Model downloaded successfully.");
    
    Ok(model_path)
}

pub fn start_recording(state: &AudioRecorderState) -> Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("No input device available")?;
    let supported_config = device.default_input_config().context("No default config")?;
    let config: cpal::StreamConfig = supported_config.clone().into();
    
    let sample_rate = config.sample_rate;
    *state.sample_rate.lock().unwrap() = sample_rate;
    let channels = config.channels;
    
    let buffer = state.buffer.clone();
    buffer.lock().unwrap().clear();
    
    let err_fn = |err| tracing::error!("An error occurred on the input audio stream: {}", err);
    
    let stream = match supported_config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config.clone(),
            move |data: &[f32], _| {
                let mut buf = buffer.lock().unwrap();
                for frame in data.chunks(channels as usize) {
                    let sum: f32 = frame.iter().sum();
                    buf.push(sum / channels as f32);
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            config.clone(),
            move |data: &[i16], _| {
                let mut buf = buffer.lock().unwrap();
                for frame in data.chunks(channels as usize) {
                    let sum: f32 = frame.iter().map(|&s| s as f32 / i16::MAX as f32).sum();
                    buf.push(sum / channels as f32);
                }
            },
            err_fn,
            None,
        )?,
        _ => anyhow::bail!("Unsupported sample format"),
    };
    
    stream.play()?;
    *state.stream.lock().unwrap() = Some(stream);
    Ok(())
}

pub fn stop_recording(state: &AudioRecorderState) -> (Vec<f32>, u32) {
    if let Some(stream) = state.stream.lock().unwrap().take() {
        let _ = stream.pause();
    }
    let data = state.buffer.lock().unwrap().clone();
    let sr = *state.sample_rate.lock().unwrap();
    (data, sr)
}

fn resample_to_16khz(input: &[f32], input_sr: u32) -> Result<Vec<f32>> {
    if input_sr == 16000 {
        return Ok(input.to_vec());
    }
    
    let ratio = input_sr as f32 / 16000.0;
    
    // Anti-aliasing low-pass filter (Moving Average)
    let window_size = ratio.ceil() as usize;
    let mut filtered_input = Vec::with_capacity(input.len());
    let mut sum = 0.0;
    for i in 0..input.len() {
        sum += input[i];
        if i >= window_size {
            sum -= input[i - window_size];
            filtered_input.push(sum / window_size as f32);
        } else {
            filtered_input.push(sum / (i + 1) as f32);
        }
    }
    
    // Linear interpolation on the filtered signal
    let out_len = (filtered_input.len() as f32 / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    
    for i in 0..out_len {
        let in_idx = i as f32 * ratio;
        let in_idx_floor = in_idx.floor() as usize;
        let in_idx_ceil = (in_idx_floor + 1).min(filtered_input.len().saturating_sub(1));
        let frac = in_idx - in_idx_floor as f32;
        
        let sample = filtered_input[in_idx_floor] * (1.0 - frac) + filtered_input[in_idx_ceil] * frac;
        out.push(sample);
    }
    
    Ok(out)
}

pub async fn transcribe_audio(app_data_dir: PathBuf, audio_data: Vec<f32>, sample_rate: u32) -> Result<String> {
    let model_path = ensure_model_downloaded(&app_data_dir).await?;
    
    let text = tokio::task::spawn_blocking(move || -> Result<String> {
        let audio_16k = resample_to_16khz(&audio_data, sample_rate)?;
        
        let model_path_str = model_path.to_str().context("Invalid characters in model path")?;
        let ctx = WhisperContext::new_with_params(model_path_str, WhisperContextParameters::default())
            .context("Failed to load Whisper model")?;

        let mut state = ctx.create_state().context("Failed to create Whisper state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(true);

        state.full(params, &audio_16k).context("Failed to run transcription")?;

        let num_segments = state.full_n_segments();
        let mut full_text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(text) = segment.to_str_lossy() {
                    full_text.push_str(&text);
                }
            }
        }
        Ok(full_text.trim().to_string())
    }).await??;

    Ok(text)
}
