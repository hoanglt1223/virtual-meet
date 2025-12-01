//! Audio Processor Module
//!
//! Real-time audio processing with volume control, mute functionality,
//! audio effects, and format conversion

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicF32, Ordering};
use std::time::{Duration, Instant};
use tracing::{info, error, warn, debug};

use crate::audio::{AudioFrameData, AudioConfig, AudioSampleFormat, AudioConverter};

/// Audio processor for real-time audio manipulation
pub struct AudioProcessor {
    config: AudioConfig,
    is_muted: Arc<AtomicBool>,
    volume: Arc<AtomicF32>,
    processed_frames: u64,
    total_samples_processed: u64,
}

impl AudioProcessor {
    /// Create a new audio processor with the given configuration
    pub fn new(config: AudioConfig) -> Self {
        Self {
            volume: Arc::new(AtomicF32::new(config.volume)),
            is_muted: Arc::new(AtomicBool::new(config.muted)),
            config,
            processed_frames: 0,
            total_samples_processed: 0,
        }
    }

    /// Process an audio frame with current effects applied
    pub fn process_frame(&mut self, mut frame: AudioFrameData) -> Result<AudioFrameData> {
        // Apply volume and mute effects
        self.apply_effects(&mut frame)?;

        // Convert to target format if needed
        if frame.sample_format != self.config.output_format {
            frame.data = AudioConverter::convert_samples(
                &frame.data,
                frame.sample_format,
                self.config.output_format,
            )?;
            frame.sample_format = self.config.output_format;
        }

        // Resample if needed
        if frame.sample_rate != self.config.output_sample_rate {
            frame.data = AudioConverter::resample(
                &frame.data,
                frame.sample_rate,
                self.config.output_sample_rate,
                frame.channels,
                frame.sample_format,
            )?;
            frame.sample_rate = self.config.output_sample_rate;
        }

        // Update statistics
        self.processed_frames += 1;
        self.total_samples_processed += frame.sample_count() as u64;

        Ok(frame)
    }

    /// Apply volume and mute effects to audio frame
    fn apply_effects(&self, frame: &mut AudioFrameData) -> Result<()> {
        if self.is_muted.load(Ordering::Relaxed) {
            // Mute - set all samples to zero
            for byte in frame.data.iter_mut() {
                *byte = 0;
            }
            return Ok(());
        }

        let volume = self.volume.load(Ordering::Relaxed);
        if volume == 1.0 {
            return Ok(()); // No volume change needed
        }

        // Apply volume scaling
        match frame.sample_format {
            AudioSampleFormat::F32 => {
                let samples: &mut [f32] = unsafe {
                    std::slice::from_raw_parts_mut(
                        frame.data.as_mut_ptr() as *mut f32,
                        frame.data.len() / 4,
                    )
                };

                for sample in samples.iter_mut() {
                    *sample *= volume;
                }
            }
            AudioSampleFormat::I16 => {
                let samples: &mut [i16] = unsafe {
                    std::slice::from_raw_parts_mut(
                        frame.data.as_mut_ptr() as *mut i16,
                        frame.data.len() / 2,
                    )
                };

                for sample in samples.iter_mut() {
                    *sample = (*sample as f32 * volume) as i16;
                }
            }
            AudioSampleFormat::U16 => {
                let samples: &mut [u16] = unsafe {
                    std::slice::from_raw_parts_mut(
                        frame.data.as_mut_ptr() as *mut u16,
                        frame.data.len() / 2,
                    )
                };

                for sample in samples.iter_mut() {
                    *sample = (*sample as f32 * volume) as u16;
                }
            }
            _ => {
                return Err(anyhow!("Volume control not implemented for format: {:?}", frame.sample_format));
            }
        }

        Ok(())
    }

    /// Set the volume level (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(anyhow!("Volume must be between 0.0 and 1.0"));
        }

        self.volume.store(volume, Ordering::Relaxed);
        info!("Volume set to: {:.2}", volume);
        Ok(())
    }

    /// Get the current volume level
    pub fn get_volume(&self) -> f32 {
        self.volume.load(Ordering::Relaxed)
    }

    /// Set mute state
    pub fn set_muted(&self, muted: bool) {
        self.is_muted.store(muted, Ordering::Relaxed);
        info!("Mute state set to: {}", muted);
    }

    /// Get current mute state
    pub fn is_muted(&self) -> bool {
        self.is_muted.load(Ordering::Relaxed)
    }

    /// Toggle mute state
    pub fn toggle_mute(&self) -> bool {
        let current_muted = self.is_muted.load(Ordering::Relaxed);
        let new_muted = !current_muted;
        self.is_muted.store(new_muted, Ordering::Relaxed);
        info!("Mute toggled to: {}", new_muted);
        new_muted
    }

    /// Get processor configuration
    pub fn get_config(&self) -> &AudioConfig {
        &self.config
    }

    /// Update processor configuration
    pub fn update_config(&mut self, config: AudioConfig) {
        self.volume.store(config.volume, Ordering::Relaxed);
        self.is_muted.store(config.muted, Ordering::Relaxed);
        self.config = config;
        info!("Audio processor configuration updated");
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> AudioProcessorStats {
        AudioProcessorStats {
            processed_frames: self.processed_frames,
            total_samples_processed: self.total_samples_processed,
            current_volume: self.get_volume(),
            is_muted: self.is_muted(),
        }
    }

    /// Reset processing statistics
    pub fn reset_stats(&mut self) {
        self.processed_frames = 0;
        self.total_samples_processed = 0;
        info!("Audio processor statistics reset");
    }
}

/// Audio processing statistics
#[derive(Debug, Clone)]
pub struct AudioProcessorStats {
    pub processed_frames: u64,
    pub total_samples_processed: u64,
    pub current_volume: f32,
    pub is_muted: bool,
}

/// Audio visualizer for audio level monitoring
pub struct AudioVisualizer {
    peak_levels: Vec<f32>,
    rms_levels: Vec<f32>,
    window_size: usize,
    channel_count: usize,
}

impl AudioVisualizer {
    /// Create a new audio visualizer
    pub fn new(channel_count: usize, window_size: usize) -> Self {
        Self {
            peak_levels: vec![0.0; channel_count],
            rms_levels: vec![0.0; channel_count],
            window_size,
            channel_count,
        }
    }

    /// Analyze audio frame and update visualization data
    pub fn analyze_frame(&mut self, frame: &AudioFrameData) -> Result<AudioVisualizationData> {
        if frame.channels as usize != self.channel_count {
            return Err(anyhow!("Channel count mismatch"));
        }

        match frame.sample_format {
            AudioSampleFormat::F32 => {
                self.analyze_f32_frame(frame)
            }
            AudioSampleFormat::I16 => {
                self.analyze_i16_frame(frame)
            }
            _ => Err(anyhow!("Visualization not supported for format: {:?}", frame.sample_format)),
        }
    }

    fn analyze_f32_frame(&mut self, frame: &AudioFrameData) -> Result<AudioVisualizationData> {
        let samples: &[f32] = unsafe {
            std::slice::from_raw_parts(
                frame.data.as_ptr() as *const f32,
                frame.data.len() / 4,
            )
        };

        let samples_per_channel = samples.len() / self.channel_count;
        let mut channel_peaks = vec![0.0; self.channel_count];
        let mut channel_rms = vec![0.0; self.channel_count];

        for ch in 0..self.channel_count {
            let mut sum_sq = 0.0;
            let mut peak = 0.0;

            for i in 0..samples_per_channel.min(self.window_size) {
                let sample = samples[i * self.channel_count + ch].abs();
                peak = peak.max(sample);
                sum_sq += sample * sample;
            }

            let rms = if samples_per_channel > 0 {
                (sum_sq / samples_per_channel.min(self.window_size) as f32).sqrt()
            } else {
                0.0
            };

            channel_peaks[ch] = peak;
            channel_rms[ch] = rms;
        }

        // Update rolling averages
        for ch in 0..self.channel_count {
            self.peak_levels[ch] = self.peak_levels[ch] * 0.7 + channel_peaks[ch] * 0.3;
            self.rms_levels[ch] = self.rms_levels[ch] * 0.7 + channel_rms[ch] * 0.3;
        }

        Ok(AudioVisualizationData {
            peak_levels: self.peak_levels.clone(),
            rms_levels: self.rms_levels.clone(),
            sample_rate: frame.sample_rate,
            channels: frame.channels,
        })
    }

    fn analyze_i16_frame(&mut self, frame: &AudioFrameData) -> Result<AudioVisualizationData> {
        let samples: &[i16] = unsafe {
            std::slice::from_raw_parts(
                frame.data.as_ptr() as *const i16,
                frame.data.len() / 2,
            )
        };

        let samples_per_channel = samples.len() / self.channel_count;
        let mut channel_peaks = vec![0.0; self.channel_count];
        let mut channel_rms = vec![0.0; self.channel_count];

        for ch in 0..self.channel_count {
            let mut sum_sq = 0.0;
            let mut peak = 0.0;

            for i in 0..samples_per_channel.min(self.window_size) {
                let sample = (samples[i * self.channel_count + ch] as f32 / 32768.0).abs();
                peak = peak.max(sample);
                sum_sq += sample * sample;
            }

            let rms = if samples_per_channel > 0 {
                (sum_sq / samples_per_channel.min(self.window_size) as f32).sqrt()
            } else {
                0.0
            };

            channel_peaks[ch] = peak;
            channel_rms[ch] = rms;
        }

        // Update rolling averages
        for ch in 0..self.channel_count {
            self.peak_levels[ch] = self.peak_levels[ch] * 0.7 + channel_peaks[ch] * 0.3;
            self.rms_levels[ch] = self.rms_levels[ch] * 0.7 + channel_rms[ch] * 0.3;
        }

        Ok(AudioVisualizationData {
            peak_levels: self.peak_levels.clone(),
            rms_levels: self.rms_levels.clone(),
            sample_rate: frame.sample_rate,
            channels: frame.channels,
        })
    }

    /// Get current visualization data
    pub fn get_current_data(&self) -> AudioVisualizationData {
        AudioVisualizationData {
            peak_levels: self.peak_levels.clone(),
            rms_levels: self.rms_levels.clone(),
            sample_rate: 0, // Not tracked here
            channels: self.channel_count as u32,
        }
    }

    /// Reset visualization levels
    pub fn reset(&mut self) {
        for level in &mut self.peak_levels {
            *level = 0.0;
        }
        for level in &mut self.rms_levels {
            *level = 0.0;
        }
    }
}

/// Audio visualization data
#[derive(Debug, Clone)]
pub struct AudioVisualizationData {
    pub peak_levels: Vec<f32>,
    pub rms_levels: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{AudioSampleFormat, AudioConfig};

    #[test]
    fn test_audio_processor_creation() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(config);
        assert_eq!(processor.get_volume(), 1.0);
        assert!(!processor.is_muted());
    }

    #[test]
    fn test_volume_control() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(config);

        // Test volume setting
        processor.set_volume(0.5).unwrap();
        assert_eq!(processor.get_volume(), 0.5);

        // Test invalid volume
        assert!(processor.set_volume(1.5).is_err());
        assert!(processor.set_volume(-0.1).is_err());
    }

    #[test]
    fn test_mute_control() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(config);

        // Test mute toggle
        let muted = processor.toggle_mute();
        assert!(muted);
        assert!(processor.is_muted());

        let muted = processor.toggle_mute();
        assert!(!muted);
        assert!(!processor.is_muted());

        // Test explicit mute setting
        processor.set_muted(true);
        assert!(processor.is_muted());
    }

    #[test]
    fn test_stats() {
        let config = AudioConfig::default();
        let mut processor = AudioProcessor::new(config);

        let stats = processor.get_stats();
        assert_eq!(stats.processed_frames, 0);
        assert_eq!(stats.total_samples_processed, 0);

        processor.reset_stats();
        let stats = processor.get_stats();
        assert_eq!(stats.processed_frames, 0);
    }

    #[test]
    fn test_audio_visualizer() {
        let mut visualizer = AudioVisualizer::new(2, 512);

        // Create test audio frame
        let test_data = vec![0x3f, 0x80, 0x00, 0x00; 1024]; // f32 samples
        let frame = AudioFrameData::new(
            test_data,
            2,
            44100,
            AudioSampleFormat::F32,
            Duration::from_millis(0),
            0,
            Duration::from_millis(23),
        );

        let viz_data = visualizer.analyze_frame(&frame).unwrap();
        assert_eq!(viz_data.channels, 2);
        assert_eq!(viz_data.peak_levels.len(), 2);
        assert_eq!(viz_data.rms_levels.len(), 2);

        visualizer.reset();
        let current_data = visualizer.get_current_data();
        assert_eq!(current_data.channels, 2);
    }

    #[test]
    fn test_audio_frame_processing() {
        let config = AudioConfig::default();
        let mut processor = AudioProcessor::new(config);

        // Create test audio frame
        let test_data = vec![0x3f, 0x80, 0x00, 0x00; 1024]; // f32 samples with value 1.0
        let frame = AudioFrameData::new(
            test_data,
            2,
            44100,
            AudioSampleFormat::F32,
            Duration::from_millis(0),
            0,
            Duration::from_millis(23),
        );

        // Process with volume 0.5
        processor.set_volume(0.5).unwrap();
        let processed = processor.process_frame(frame).unwrap();

        // Check that samples were scaled
        let processed_samples: &[f32] = unsafe {
            std::slice::from_raw_parts(
                processed.data.as_ptr() as *const f32,
                processed.data.len() / 4,
            )
        };

        assert!((processed_samples[0] - 0.5).abs() < 0.001);
    }
}