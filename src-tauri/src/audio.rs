//! Audio Processing Module
//!
//! Core audio playback system with MP3/PCM decoding, resampling,
//! format conversion, volume control, and mute functionality

use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use std::collections::VecDeque;
use tracing::{info, error, warn, debug};

// Audio decoding and processing
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// Audio resampling and conversion
use rodio::{Sample, Source, OutputStream, OutputStreamHandle};
use cpal::SampleFormat;

/// Audio sample formats supported by the pipeline
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioSampleFormat {
    F32,
    I16,
    U16,
    I24,
    I32,
}

impl From<SampleFormat> for AudioSampleFormat {
    fn from(format: SampleFormat) -> Self {
        match format {
            SampleFormat::F32 => AudioSampleFormat::F32,
            SampleFormat::I16 => AudioSampleFormat::I16,
            SampleFormat::U16 => AudioSampleFormat::U16,
            SampleFormat::I64 => AudioSampleFormat::I32, // Map I64 to I32 for now
            _ => AudioSampleFormat::F32, // Default to F32
        }
    }
}

/// Audio frame data with metadata
#[derive(Debug, Clone)]
pub struct AudioFrameData {
    pub data: Vec<u8>,
    pub channels: u32,
    pub sample_rate: u32,
    pub sample_format: AudioSampleFormat,
    pub timestamp: Duration,
    pub frame_number: u64,
    pub duration: Duration,
}

impl AudioFrameData {
    pub fn new(
        data: Vec<u8>,
        channels: u32,
        sample_rate: u32,
        sample_format: AudioSampleFormat,
        timestamp: Duration,
        frame_number: u64,
        duration: Duration,
    ) -> Self {
        Self {
            data,
            channels,
            sample_rate,
            sample_format,
            timestamp,
            frame_number,
            duration,
        }
    }

    /// Get the number of samples in this frame
    pub fn sample_count(&self) -> usize {
        let bytes_per_sample = self.sample_format.bytes_per_sample();
        self.data.len() / (bytes_per_sample * self.channels as usize)
    }

    /// Get the duration of this frame
    pub fn frame_duration(&self) -> Duration {
        self.duration
    }

    /// Get the size of the frame in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl AudioSampleFormat {
    /// Get the number of bytes per sample for this format
    pub fn bytes_per_sample(self) -> usize {
        match self {
            AudioSampleFormat::F32 => 4,
            AudioSampleFormat::I16 => 2,
            AudioSampleFormat::U16 => 2,
            AudioSampleFormat::I24 => 3,
            AudioSampleFormat::I32 => 4,
        }
    }

    /// Get the name of this format
    pub fn name(self) -> &'static str {
        match self {
            AudioSampleFormat::F32 => "f32",
            AudioSampleFormat::I16 => "i16",
            AudioSampleFormat::U16 => "u16",
            AudioSampleFormat::I24 => "i24",
            AudioSampleFormat::I32 => "i32",
        }
    }
}

/// Audio buffer for managing decoded audio frames
pub struct AudioBuffer {
    frames: Arc<StdMutex<VecDeque<AudioFrameData>>>,
    max_size: usize,
    total_frames: u64,
}

impl AudioBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            frames: Arc::new(StdMutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            total_frames: 0,
        }
    }

    /// Add a frame to the buffer
    pub fn push_frame(&mut self, frame: AudioFrameData) -> Result<()> {
        let mut frames = self.frames.lock().map_err(|_| anyhow!("Failed to lock audio buffer"))?;

        if frames.len() >= self.max_size {
            frames.pop_front(); // Remove oldest frame
        }

        frames.push_back(frame);
        self.total_frames += 1;

        Ok(())
    }

    /// Get the next frame for playback
    pub fn get_next_frame(&self) -> Option<AudioFrameData> {
        let frames = self.frames.lock().ok()?;
        frames.front().cloned()
    }

    /// Remove and return the next frame
    pub fn pop_frame(&mut self) -> Option<AudioFrameData> {
        let mut frames = self.frames.lock().ok()?;
        frames.pop_front()
    }

    /// Clear all frames from buffer
    pub fn clear(&mut self) -> Result<()> {
        let mut frames = self.frames.lock().map_err(|_| anyhow!("Failed to lock audio buffer"))?;
        frames.clear();
        self.total_frames = 0;
        Ok(())
    }

    /// Get the number of frames currently in the buffer
    pub fn len(&self) -> usize {
        let frames = self.frames.lock().ok();
        match frames {
            Some(f) => f.len(),
            None => 0,
        }
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the total number of frames that have passed through the buffer
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Get the maximum buffer size
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

/// Audio metadata extracted from media files
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub duration: Option<Duration>,
    pub channels: u32,
    pub sample_rate: u32,
    pub format: String,
    pub codec: String,
    pub bit_rate: Option<u64>,
}

impl AudioMetadata {
    pub fn new() -> Self {
        Self {
            duration: None,
            channels: 0,
            sample_rate: 0,
            format: String::new(),
            codec: String::new(),
            bit_rate: None,
        }
    }
}

impl Default for AudioMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio configuration settings
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub output_sample_rate: u32,
    pub output_channels: u32,
    pub output_format: AudioSampleFormat,
    pub buffer_size: usize,
    pub volume: f32,
    pub muted: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            output_sample_rate: 44100,
            output_channels: 2,
            output_format: AudioSampleFormat::F32,
            buffer_size: 1024 * 8, // 8KB buffer
            volume: 1.0,
            muted: false,
        }
    }
}

/// Audio validation utilities
pub struct AudioValidator;

impl AudioValidator {
    /// Validate if a file is a supported audio format
    pub fn validate_audio_file<P: AsRef<std::path::Path>>(path: P) -> Result<()> {
        let path = path.as_ref();

        // Check file extension
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("No file extension found"))?;

        let supported_formats = ["mp3", "wav", "m4a", "aac", "ogg", "flac"];
        if !supported_formats.contains(&extension.to_lowercase().as_str()) {
            return Err(anyhow!("Unsupported audio format: {}", extension));
        }

        // Check if file exists and is readable
        if !path.exists() {
            return Err(anyhow!("Audio file does not exist: {}", path.display()));
        }

        Ok(())
    }

    /// Get supported audio formats
    pub fn supported_formats() -> Vec<&'static str> {
        vec!["mp3", "wav", "m4a", "aac", "ogg", "flac"]
    }
}

/// Audio format conversion utilities
pub struct AudioConverter;

impl AudioConverter {
    /// Convert audio samples between different formats
    pub fn convert_samples(
        input_data: &[u8],
        input_format: AudioSampleFormat,
        output_format: AudioSampleFormat,
    ) -> Result<Vec<u8>> {
        if input_format == output_format {
            return Ok(input_data.to_vec());
        }

        // For now, implement basic conversions
        match (input_format, output_format) {
            (AudioSampleFormat::I16, AudioSampleFormat::F32) => {
                let samples: &[i16] = unsafe {
                    std::slice::from_raw_parts(
                        input_data.as_ptr() as *const i16,
                        input_data.len() / 2,
                    )
                };

                let output: Vec<u8> = samples
                    .iter()
                    .flat_map(|&sample| {
                        let f32_sample = sample as f32 / 32768.0;
                        f32_sample.to_ne_bytes().to_vec()
                    })
                    .collect();

                Ok(output)
            }
            (AudioSampleFormat::F32, AudioSampleFormat::I16) => {
                let samples: &[f32] = unsafe {
                    std::slice::from_raw_parts(
                        input_data.as_ptr() as *const f32,
                        input_data.len() / 4,
                    )
                };

                let output: Vec<u8> = samples
                    .iter()
                    .flat_map(|&sample| {
                        let i16_sample = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                        i16_sample.to_ne_bytes().to_vec()
                    })
                    .collect();

                Ok(output)
            }
            _ => Err(anyhow!("Unsupported format conversion: {:?} -> {:?}", input_format, output_format)),
        }
    }

    /// Resample audio data to a different sample rate
    pub fn resample(
        input_data: &[u8],
        input_sample_rate: u32,
        output_sample_rate: u32,
        channels: u32,
        sample_format: AudioSampleFormat,
    ) -> Result<Vec<u8>> {
        if input_sample_rate == output_sample_rate {
            return Ok(input_data.to_vec());
        }

        // Simple linear interpolation resampling
        // In a production system, you'd want to use a more sophisticated resampling algorithm
        let ratio = output_sample_rate as f64 / input_sample_rate as f64;
        let samples_per_channel = input_data.len() / (sample_format.bytes_per_sample() * channels as usize);
        let output_samples_per_channel = (samples_per_channel as f64 * ratio) as usize;

        match sample_format {
            AudioSampleFormat::F32 => {
                let input_samples: &[f32] = unsafe {
                    std::slice::from_raw_parts(
                        input_data.as_ptr() as *const f32,
                        input_data.len() / 4,
                    )
                };

                let mut output_samples = vec![0.0f32; output_samples_per_channel * channels as usize];

                for ch in 0..channels {
                    for i in 0..output_samples_per_channel {
                        let input_index = (i as f64 / ratio) as usize;
                        let next_index = (input_index + 1).min(samples_per_channel - 1);
                        let fraction = (i as f64 / ratio) - input_index as f64;

                        let input_val = input_samples[input_index * channels as usize + ch as usize];
                        let next_val = input_samples[next_index * channels as usize + ch as usize];

                        let interpolated = input_val + fraction * (next_val - input_val);
                        output_samples[i * channels as usize + ch as usize] = interpolated;
                    }
                }

                let output_bytes: Vec<u8> = unsafe {
                    std::slice::from_raw_parts(
                        output_samples.as_ptr() as *const u8,
                        output_samples.len() * 4,
                    )
                }.to_vec();

                Ok(output_bytes)
            }
            _ => Err(anyhow!("Resampling not implemented for format: {:?}", sample_format)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_buffer_operations() {
        let mut buffer = AudioBuffer::new(3);

        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        let frame = AudioFrameData::new(
            vec![1, 2, 3, 4],
            2,
            44100,
            AudioSampleFormat::I16,
            Duration::from_millis(0),
            0,
            Duration::from_millis(23),
        );

        buffer.push_frame(frame.clone()).unwrap();
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());

        let retrieved = buffer.get_next_frame().unwrap();
        assert_eq!(retrieved.channels, 2);
        assert_eq!(retrieved.sample_rate, 44100);

        let popped = buffer.pop_frame().unwrap();
        assert_eq!(popped.channels, 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_audio_validation() {
        assert!(AudioValidator::validate_audio_file("test.mp3").is_ok());
        assert!(AudioValidator::validate_audio_file("test.wav").is_ok());
        assert!(AudioValidator::validate_audio_file("test.xyz").is_err());

        let formats = AudioValidator::supported_formats();
        assert!(formats.contains(&"mp3"));
        assert!(formats.contains(&"wav"));
    }

    #[test]
    fn test_audio_conversion() {
        let input_data = [0x00, 0x01, 0x00, 0x02]; // Two i16 samples: 256, 512
        let output = AudioConverter::convert_samples(
            &input_data,
            AudioSampleFormat::I16,
            AudioSampleFormat::F32,
        ).unwrap();

        assert_eq!(output.len(), 8); // Two f32 samples = 8 bytes
    }

    #[test]
    fn test_audio_metadata() {
        let metadata = AudioMetadata::new();
        assert_eq!(metadata.channels, 0);
        assert_eq!(metadata.sample_rate, 0);
        assert!(metadata.duration.is_none());
    }

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.output_sample_rate, 44100);
        assert_eq!(config.output_channels, 2);
        assert_eq!(config.output_format, AudioSampleFormat::F32);
        assert_eq!(config.volume, 1.0);
        assert!(!config.muted);
    }

    #[test]
    fn test_audio_sample_format() {
        assert_eq!(AudioSampleFormat::F32.bytes_per_sample(), 4);
        assert_eq!(AudioSampleFormat::I16.bytes_per_sample(), 2);
        assert_eq!(AudioSampleFormat::U16.bytes_per_sample(), 2);
        assert_eq!(AudioSampleFormat::I32.bytes_per_sample(), 4);

        assert_eq!(AudioSampleFormat::F32.name(), "f32");
        assert_eq!(AudioSampleFormat::I16.name(), "i16");
    }
}