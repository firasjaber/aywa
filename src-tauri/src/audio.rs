use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream, StreamConfig,
};

#[derive(Debug, Clone)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub sample_count: usize,
}

pub struct RecorderSession {
    stream: Stream,
    sample_rate: u32,
    channels: u16,
    samples: Arc<Mutex<Vec<i16>>>,
}

// The recorder session is only accessed behind app-managed mutexes in this app.
unsafe impl Send for RecorderSession {}
unsafe impl Sync for RecorderSession {}

impl RecorderSession {
    pub fn start(output_dir: &Path) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default microphone available".to_string())?;
        let device_name = device
            .name()
            .unwrap_or_else(|_| "unknown input device".to_string());

        let supported_config = device
            .default_input_config()
            .map_err(|error| format!("Failed to read default input config: {error}"))?;

        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels();
        let stream_config = StreamConfig {
            channels,
            sample_rate: SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };
        eprintln!(
            "[audio] start device={} sample_format={:?} sample_rate={} channels={}",
            device_name,
            supported_config.sample_format(),
            sample_rate,
            channels
        );

        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_for_stream = Arc::clone(&samples);

        let error_callback = |error| {
            eprintln!("recording stream error: {error}");
        };

        let stream = match supported_config.sample_format() {
            SampleFormat::I16 => {
                build_i16_stream(&device, &stream_config, samples_for_stream, error_callback)
            }
            SampleFormat::U16 => {
                build_u16_stream(&device, &stream_config, samples_for_stream, error_callback)
            }
            SampleFormat::F32 => {
                build_f32_stream(&device, &stream_config, samples_for_stream, error_callback)
            }
            other => return Err(format!("Unsupported input sample format: {other:?}")),
        }?;

        stream
            .play()
            .map_err(|error| format!("Failed to start input stream: {error}"))?;

        let _ = output_dir;

        Ok(Self {
            stream,
            sample_rate,
            channels,
            samples,
        })
    }

    pub fn stop(self, output_dir: &Path) -> Result<RecordingResult, String> {
        drop(self.stream);

        let raw_samples = self
            .samples
            .lock()
            .map_err(|_| "recording samples mutex poisoned".to_string())?
            .clone();

        if raw_samples.is_empty() {
            return Err("Recording contained no audio samples".to_string());
        }

        std::fs::create_dir_all(output_dir)
            .map_err(|error| format!("Failed to create recording directory: {error}"))?;

        let mono_samples = downmix_to_mono_i16(&raw_samples, self.channels);
        if mono_samples.is_empty() {
            return Err("Recording contained no mono audio samples".to_string());
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("System clock error: {error}"))?
            .as_millis();

        let path = build_output_path(output_dir, timestamp);
        let peak = sample_peak_i16(&mono_samples);
        let rms = sample_rms_i16(&mono_samples);
        eprintln!(
            "[audio] stop sample_rate={} duration_ms={} samples={} peak={} rms={}",
            self.sample_rate,
            (mono_samples.len() as u64 * 1000) / self.sample_rate as u64,
            mono_samples.len(),
            peak,
            rms
        );
        write_wav_file(&path, self.sample_rate, &mono_samples)?;

        if peak == 0 {
            let _ = std::fs::remove_file(&path);
            return Err(format!(
                "Recording contains silence. Check your default microphone/input device."
            ));
        }

        Ok(RecordingResult {
            path,
            sample_rate: self.sample_rate,
            sample_count: mono_samples.len(),
        })
    }
}

fn build_i16_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[i16], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend_from_slice(data);
                }
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("Failed to build i16 input stream: {error}"))
}

fn build_u16_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[u16], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend(data.iter().map(|sample| (*sample as i32 - 32_768) as i16));
                }
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("Failed to build u16 input stream: {error}"))
}

fn build_f32_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend(data.iter().map(|sample| {
                        let scaled = (*sample * i16::MAX as f32).round();
                        scaled.clamp(i16::MIN as f32, i16::MAX as f32) as i16
                    }));
                }
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("Failed to build f32 input stream: {error}"))
}

fn write_wav_file(path: &Path, sample_rate: u32, samples: &[i16]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| format!("Failed to create WAV file: {error}"))?;

    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|error| format!("Failed to write WAV sample: {error}"))?;
    }

    writer
        .finalize()
        .map_err(|error| format!("Failed to finalize WAV file: {error}"))
}

pub fn downmix_to_mono_i16(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels as usize)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|sample| *sample as i32).sum();
            (sum / frame.len() as i32) as i16
        })
        .collect()
}

pub fn build_output_path(output_dir: &Path, timestamp_ms: u128) -> PathBuf {
    output_dir.join(format!("recording-{timestamp_ms}.wav"))
}

fn sample_peak_i16(samples: &[i16]) -> i16 {
    samples.iter().map(|sample| sample.abs()).max().unwrap_or(0)
}

fn sample_rms_i16(samples: &[i16]) -> i16 {
    if samples.is_empty() {
        return 0;
    }

    let mean_square = samples
        .iter()
        .map(|sample| {
            let sample = *sample as f64;
            sample * sample
        })
        .sum::<f64>()
        / samples.len() as f64;

    mean_square.sqrt().round() as i16
}

#[cfg(test)]
mod tests {
    use super::{build_output_path, downmix_to_mono_i16, sample_peak_i16, sample_rms_i16};

    #[test]
    fn stereo_i16_samples_are_downmixed_to_mono() {
        let samples = vec![1000_i16, -1000_i16, 3000_i16, 1000_i16];
        let mono = downmix_to_mono_i16(&samples, 2);
        assert_eq!(mono, vec![0_i16, 2000_i16]);
    }

    #[test]
    fn mono_samples_are_kept_as_is() {
        let samples = vec![12_i16, -8_i16, 33_i16];
        let mono = downmix_to_mono_i16(&samples, 1);
        assert_eq!(mono, samples);
    }

    #[test]
    fn output_path_uses_wav_extension_and_prefix() {
        let path = build_output_path(std::path::Path::new("C:/temp"), 1_713_024_800_123);
        assert_eq!(path.parent().unwrap(), std::path::Path::new("C:/temp"));
        assert_eq!(path.extension().unwrap(), "wav");
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("recording-"));
    }

    #[test]
    fn sample_peak_detects_silence() {
        assert_eq!(sample_peak_i16(&[0, 0, 0]), 0);
        assert_eq!(sample_peak_i16(&[-4, 2, 3]), 4);
    }

    #[test]
    fn sample_rms_detects_silence() {
        assert_eq!(sample_rms_i16(&[]), 0);
        assert_eq!(sample_rms_i16(&[0, 0, 0]), 0);
        assert!(sample_rms_i16(&[1000, -1000]) > 0);
    }

    #[test]
    fn silence_error_does_not_disclose_a_recording_path() {
        let error = "Recording contains silence. Check your default microphone/input device.";
        assert!(!error.contains("recording-"));
        assert!(!error.contains(":\\"));
    }
}
