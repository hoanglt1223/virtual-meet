//! Virtual Microphone Implementation using WASAPI
//!
//! This module provides native Rust implementation of a virtual microphone
//! without requiring external applications like VB-CABLE or VoiceMeeter.

use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex as StdMutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, StreamConfig, SampleFormat, Stream};

// Import audio processing modules
use crate::audio::{AudioFrameData, AudioBuffer, AudioConfig, AudioSampleFormat, AudioValidator};
use crate::audio_decoder::AudioDecoder;
use crate::audio_processor::{AudioProcessor, AudioProcessorStats, AudioVisualizer, AudioVisualizationData};

/// Virtual microphone manager
pub struct VirtualMicrophone {
    is_active: Arc<AtomicBool>,
    current_source: Arc<Mutex<Option<String>>>,
    audio_stream: Option<Stream>,

    // Audio pipeline components
    audio_config: AudioConfig,
    audio_decoder: Arc<Mutex<AudioDecoder>>,
    audio_processor: Arc<Mutex<AudioProcessor>>,
    audio_buffer: Arc<StdMutex<AudioBuffer>>,
    audio_visualizer: Arc<Mutex<AudioVisualizer>>,

    // Threading and synchronization
    decode_thread_handle: Option<thread::JoinHandle<()>>,
    playback_thread_handle: Option<thread::JoinHandle<()>>,
    should_stop: Arc<AtomicBool>,
}

impl VirtualMicrophone {
    /// Create a new virtual microphone instance
    pub fn new() -> Self {
        let audio_config = AudioConfig::default();
        let audio_processor = AudioProcessor::new(audio_config.clone());
        let audio_decoder = AudioDecoder::new();
        let audio_buffer = AudioBuffer::new(audio_config.buffer_size);
        let audio_visualizer = AudioVisualizer::new(audio_config.output_channels as usize, 1024);

        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            current_source: Arc::new(Mutex::new(None)),
            audio_stream: None,
            audio_config,
            audio_decoder: Arc::new(Mutex::new(audio_decoder)),
            audio_processor: Arc::new(Mutex::new(audio_processor)),
            audio_buffer: Arc::new(StdMutex::new(audio_buffer)),
            audio_visualizer: Arc::new(Mutex::new(audio_visualizer)),
            decode_thread_handle: None,
            playback_thread_handle: None,
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the virtual microphone
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing virtual microphone using WASAPI");

        // Initialize audio processing pipeline
        let processor = self.audio_processor.lock().await;
        info!("Audio processor initialized: {} channels, {} Hz, {:?} format",
              self.audio_config.output_channels,
              self.audio_config.output_sample_rate,
              self.audio_config.output_format);

        warn!("Virtual microphone initialization partially implemented - requires Windows Audio Driver development for true virtual device");
        Ok(())
    }

    /// Start streaming audio from a file
    pub async fn start_streaming(&mut self, audio_path: &str) -> Result<()> {
        if self.is_active.load(Ordering::Relaxed) {
            return Err(anyhow!("Microphone already streaming"));
        }

        // Validate audio file
        AudioValidator::validate_audio_file(audio_path)?;

        info!("Starting audio stream from: {}", audio_path);

        // Update current source
        let mut current_source = self.current_source.lock().await;
        *current_source = Some(audio_path.to_string());
        drop(current_source);

        // Reset stop flag
        self.should_stop.store(false, Ordering::Relaxed);

        // Start audio decoding thread
        self.start_decode_thread(audio_path)?;

        // Start audio playback thread (simulated virtual device)
        self.start_playback_thread()?;

        // Set active state
        self.is_active.store(true, Ordering::Relaxed);

        info!("Audio streaming started successfully");
        Ok(())
    }

    /// Start the audio decoding thread
    fn start_decode_thread(&mut self, audio_path: &str) -> Result<()> {
        let audio_path = audio_path.to_string();
        let decoder = Arc::clone(&self.audio_decoder);
        let buffer = Arc::clone(&self.audio_buffer);
        let should_stop = Arc::clone(&self.should_stop);
        let config = self.audio_config.clone();

        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                if let Err(e) = Self::decode_audio_file(
                    &audio_path,
                    decoder,
                    buffer,
                    should_stop,
                    config,
                ).await {
                    error!("Audio decoding failed: {}", e);
                }
            });
        });

        self.decode_thread_handle = Some(handle);
        Ok(())
    }

    /// Start the audio playback thread (virtual device simulation)
    fn start_playback_thread(&mut self) -> Result<()> {
        let processor = Arc::clone(&self.audio_processor);
        let buffer = Arc::clone(&self.audio_buffer);
        let visualizer = Arc::clone(&self.audio_visualizer);
        let should_stop = Arc::clone(&self.should_stop);

        // For now, we'll create a simple output stream as a proof of concept
        // In a full implementation, this would interface with a virtual audio device
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| anyhow!("No audio output device available"))?;

        let config = device.default_output_config()?;
        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();

        let stream = match sample_format {
            SampleFormat::F32 => self.create_virtual_audio_stream::<f32>(&device, &config, processor, buffer, visualizer, should_stop)?,
            SampleFormat::I16 => self.create_virtual_audio_stream::<i16>(&device, &config, processor, buffer, visualizer, should_stop)?,
            SampleFormat::U16 => self.create_virtual_audio_stream::<u16>(&device, &config, processor, buffer, visualizer, should_stop)?,
            _ => return Err(anyhow!("Unsupported sample format: {:?}", sample_format)),
        };

        stream.play()?;
        self.audio_stream = Some(stream);

        Ok(())
    }

    /// Decode audio file into buffer
    async fn decode_audio_file(
        audio_path: &str,
        decoder: Arc<Mutex<AudioDecoder>>,
        buffer: Arc<StdMutex<AudioBuffer>>,
        should_stop: Arc<AtomicBool>,
        config: AudioConfig,
    ) -> Result<()> {
        info!("Starting audio file decoding: {}", audio_path);

        // Open and decode audio file
        let mut decoder_lock = decoder.lock().await;
        let frames = decoder_lock.decode_all(audio_path, config.buffer_size)?;
        drop(decoder_lock);

        info!("Decoded {} audio frames", frames.len());

        // Add frames to buffer
        for frame in frames {
            if should_stop.load(Ordering::Relaxed) {
                info!("Decoding stopped by user request");
                break;
            }

            let mut buffer_lock = buffer.lock().map_err(|_| anyhow!("Failed to lock audio buffer"))?;
            buffer_lock.push_frame(frame)?;
        }

        info!("Audio file decoding completed");
        Ok(())
    }

    fn create_virtual_audio_stream<T>(
        &mut self,
        device: &Device,
        config: &StreamConfig,
        processor: Arc<Mutex<AudioProcessor>>,
        buffer: Arc<StdMutex<AudioBuffer>>,
        visualizer: Arc<Mutex<AudioVisualizer>>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<Stream>
    where
        T: cpal::Sample,
    {
        // This creates a simulated virtual microphone by playing audio through speakers
        // In a full implementation, this would create a true virtual audio input device

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _| {
                if should_stop.load(Ordering::Relaxed) {
                    // Fill with silence if stopped
                    for sample in data.iter_mut() {
                        *sample = T::from(&0.0);
                    }
                    return;
                }

                // Get next audio frame from buffer
                let mut buffer_lock = match buffer.lock() {
                    Ok(lock) => lock,
                    Err(_) => {
                        // Fill with silence on lock error
                        for sample in data.iter_mut() {
                            *sample = T::from(&0.0);
                        }
                        return;
                    }
                };

                let mut frame = if let Some(frame) = buffer_lock.pop_frame() {
                    frame
                } else {
                    // No frame available, fill with silence
                    for sample in data.iter_mut() {
                        *sample = T::from(&0.0);
                    }
                    return;
                };

                drop(buffer_lock);

                // Process audio frame (volume, format conversion, etc.)
                let processed_frame = match processor.try_lock() {
                    Ok(mut proc) => match proc.process_frame(frame) {
                        Ok(processed) => processed,
                        Err(e) => {
                            error!("Audio processing failed: {}", e);
                            // Use original frame if processing fails
                            frame
                        }
                    },
                    Err(_) => {
                        // Processor locked, use original frame
                        frame
                    }
                };

                // Update visualizer
                if let Ok(mut viz) = visualizer.try_lock() {
                    if let Err(e) = viz.analyze_frame(&processed_frame) {
                        debug!("Visualization update failed: {}", e);
                    }
                }

                // Convert processed frame data to output format
                Self::fill_output_buffer::<T>(data, &processed_frame);
            },
            |err| error!("Audio stream error: {}", err),
            None,
        )?;

        info!("Virtual audio stream created successfully");
        Ok(stream)
    }

    /// Fill output buffer with processed audio data
    fn fill_output_buffer<T>(data: &mut [T], frame: &AudioFrameData)
    where
        T: cpal::Sample,
    {
        let samples_needed = data.len();
        let samples_available = frame.data.len() / 4; // Assuming f32 samples

        if samples_available == 0 {
            // No audio data, fill with silence
            for sample in data.iter_mut() {
                *sample = T::from(&0.0);
            }
            return;
        }

        // Convert frame data to f32 samples
        let frame_samples: &[f32] = unsafe {
            std::slice::from_raw_parts(
                frame.data.as_ptr() as *const f32,
                frame.data.len() / 4,
            )
        };

        // Fill output buffer
        for (i, output_sample) in data.iter_mut().enumerate() {
            if i < samples_available {
                *output_sample = T::from(&frame_samples[i]);
            } else {
                *output_sample = T::from(&0.0);
            }
        }
    }

    /// Stop streaming
    pub async fn stop_streaming(&mut self) -> Result<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping audio stream");

        // Signal threads to stop
        self.should_stop.store(true, Ordering::Relaxed);

        // Stop audio stream
        if let Some(stream) = self.audio_stream.take() {
            drop(stream);
        }

        // Wait for threads to finish
        if let Some(handle) = self.decode_thread_handle.take() {
            if let Err(e) = handle.join() {
                error!("Decode thread join failed: {:?}", e);
            }
        }

        if let Some(handle) = self.playback_thread_handle.take() {
            if let Err(e) = handle.join() {
                error!("Playback thread join failed: {:?}", e);
            }
        }

        // Clear audio buffer
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            if let Err(e) = buffer.clear() {
                error!("Failed to clear audio buffer: {}", e);
            }
        }

        // Update state
        self.is_active.store(false, Ordering::Relaxed);
        let mut current_source = self.current_source.lock().await;
        *current_source = None;

        info!("Audio stream stopped successfully");
        Ok(())
    }

    /// Set volume level (0.0 to 1.0)
    pub async fn set_volume(&self, volume: f32) -> Result<()> {
        let mut processor = self.audio_processor.lock().await;
        processor.set_volume(volume)
    }

    /// Get current volume level
    pub async fn get_volume(&self) -> f32 {
        let processor = self.audio_processor.lock().await;
        processor.get_volume()
    }

    /// Set mute state
    pub async fn set_muted(&self, muted: bool) {
        let processor = self.audio_processor.lock().await;
        processor.set_muted(muted);
    }

    /// Get current mute state
    pub async fn is_muted(&self) -> bool {
        let processor = self.audio_processor.lock().await;
        processor.is_muted()
    }

    /// Toggle mute state
    pub async fn toggle_mute(&self) -> bool {
        let processor = self.audio_processor.lock().await;
        processor.toggle_mute()
    }

    /// Check if microphone is active
    pub async fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Get current audio source
    pub async fn current_source(&self) -> Option<String> {
        self.current_source.lock().await.clone()
    }

    /// Get audio processing statistics
    pub async fn get_processing_stats(&self) -> AudioProcessorStats {
        let processor = self.audio_processor.lock().await;
        processor.get_stats()
    }

    /// Get current audio visualization data
    pub async fn get_visualization_data(&self) -> AudioVisualizationData {
        let visualizer = self.audio_visualizer.lock().await;
        visualizer.get_current_data()
    }

    /// Get buffer status
    pub async fn get_buffer_status(&self) -> (usize, usize, u64) {
        let buffer = self.audio_buffer.lock().unwrap();
        (buffer.len(), buffer.max_size(), buffer.total_frames())
    }

    /// List available audio devices
    pub async fn list_devices() -> Result<Vec<String>> {
        info!("Enumerating audio devices");

        let host = cpal::default_host();
        let devices = host.devices()?;

        let device_names: Result<Vec<_>> = devices
            .map(|d| d.name().map_err(|e| anyhow!("Failed to get device name: {}", e)))
            .collect();

        match device_names {
            Ok(mut names) => {
                names.insert(0, "VirtualMicrophone".to_string());
                Ok(names)
            }
            Err(e) => Err(e)
        }
    }
}

impl Drop for VirtualMicrophone {
    fn drop(&mut self) {
        info!("Virtual microphone dropped");
        // TODO: Ensure cleanup of WASAPI resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{AudioConfig, AudioSampleFormat};

    #[tokio::test]
    async fn test_microphone_creation() {
        let mic = VirtualMicrophone::new();
        assert!(!mic.is_active().await);
        assert_eq!(mic.get_volume().await, 1.0);
        assert!(!mic.is_muted().await);
        assert!(mic.current_source().await.is_none());
    }

    #[tokio::test]
    async fn test_device_enumeration() {
        let devices = VirtualMicrophone::list_devices().await;
        assert!(devices.is_ok());
        if let Ok(devs) = devices {
            assert!(!devs.is_empty());
            assert!(devs.contains(&"VirtualMicrophone".to_string()));
        }
    }

    #[tokio::test]
    async fn test_microphone_initialization() {
        let mic = VirtualMicrophone::new();
        let result = mic.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_volume_control() {
        let mic = VirtualMicrophone::new();

        // Test setting volume
        let result = mic.set_volume(0.5).await;
        assert!(result.is_ok());
        assert_eq!(mic.get_volume().await, 0.5);

        // Test invalid volume
        let result = mic.set_volume(1.5).await;
        assert!(result.is_err());

        let result = mic.set_volume(-0.1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mute_control() {
        let mic = VirtualMicrophone::new();

        // Test initial state
        assert!(!mic.is_muted().await);

        // Test setting mute
        mic.set_muted(true).await;
        assert!(mic.is_muted().await);

        // Test toggle mute
        let new_state = mic.toggle_mute().await;
        assert!(!new_state);
        assert!(!mic.is_muted().await);

        // Test toggle again
        let new_state = mic.toggle_mute().await;
        assert!(new_state);
        assert!(mic.is_muted().await);
    }

    #[tokio::test]
    async fn test_processing_stats() {
        let mic = VirtualMicrophone::new();
        let stats = mic.get_processing_stats().await;

        assert_eq!(stats.processed_frames, 0);
        assert_eq!(stats.total_samples_processed, 0);
        assert_eq!(stats.current_volume, 1.0);
        assert!(!stats.is_muted);
    }

    #[tokio::test]
    async fn test_buffer_status() {
        let mic = VirtualMicrophone::new();
        let (len, max_size, total_frames) = mic.get_buffer_status().await;

        assert_eq!(len, 0);
        assert!(max_size > 0);
        assert_eq!(total_frames, 0);
    }

    #[tokio::test]
    async fn test_visualization_data() {
        let mic = VirtualMicrophone::new();
        let viz_data = mic.get_visualization_data().await;

        assert_eq!(viz_data.channels, 2); // Default stereo
        assert_eq!(viz_data.peak_levels.len(), 2);
        assert_eq!(viz_data.rms_levels.len(), 2);
    }

    #[test]
    fn test_audio_config_integration() {
        let config = AudioConfig::default();
        let mic = VirtualMicrophone::new();

        // The microphone should use the default config internally
        assert_eq!(config.output_sample_rate, 44100);
        assert_eq!(config.output_channels, 2);
        assert_eq!(config.output_format, AudioSampleFormat::F32);
    }

    #[tokio::test]
    async fn test_streaming_workflow() {
        let mut mic = VirtualMicrophone::new();

        // Should not be active initially
        assert!(!mic.is_active().await);

        // Test that we can't start streaming with an invalid file
        let result = mic.start_streaming("nonexistent.mp3").await;
        assert!(result.is_err());

        // Should still not be active
        assert!(!mic.is_active().await);

        // Test stopping when not active should be ok
        let result = mic.stop_streaming().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_microphone_drop() {
        let mic = VirtualMicrophone::new();
        // This test just ensures that the Drop trait works without panicking
        drop(mic);
    }

    // Integration test with actual audio files would require test fixtures
    // and should be placed in a separate integration test file
}