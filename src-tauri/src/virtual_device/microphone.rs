//! Virtual Microphone - plays audio to a selected output device
//! When routed to VB-CABLE Input, meeting apps see it as microphone input.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SizedSample, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::audio::{AudioBuffer, AudioConfig, AudioFrameData, AudioSampleFormat};
use crate::audio_decoder::AudioDecoder;
use crate::audio_processor::{AudioProcessor, AudioProcessorStats, AudioVisualizationData, AudioVisualizer};

/// Virtual microphone that plays audio to a selected output device
pub struct VirtualMicrophone {
    is_active: Arc<AtomicBool>,
    current_source: Arc<StdMutex<Option<String>>>,
    audio_stream: Arc<StdMutex<Option<Stream>>>,
    audio_config: AudioConfig,
    audio_processor: Arc<Mutex<AudioProcessor>>,
    audio_buffer: Arc<StdMutex<AudioBuffer>>,
    audio_visualizer: Arc<Mutex<AudioVisualizer>>,
    should_stop: Arc<AtomicBool>,
    selected_device: Arc<StdMutex<Option<String>>>,
}

// SAFETY: cpal::Stream uses OS audio callbacks that are thread-safe on Windows (WASAPI).
// The stream is only accessed through the Mutex, ensuring exclusive access.
unsafe impl Send for VirtualMicrophone {}
unsafe impl Sync for VirtualMicrophone {}

impl VirtualMicrophone {
    pub fn new() -> Self {
        let audio_config = AudioConfig::default();
        let audio_processor = AudioProcessor::new(audio_config.clone());
        let audio_buffer = AudioBuffer::new(audio_config.buffer_size);
        let audio_visualizer = AudioVisualizer::new(audio_config.output_channels as usize, 1024);

        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            current_source: Arc::new(StdMutex::new(None)),
            audio_stream: Arc::new(StdMutex::new(None)),
            audio_config,
            audio_processor: Arc::new(Mutex::new(audio_processor)),
            audio_buffer: Arc::new(StdMutex::new(audio_buffer)),
            audio_visualizer: Arc::new(Mutex::new(audio_visualizer)),
            should_stop: Arc::new(AtomicBool::new(false)),
            selected_device: Arc::new(StdMutex::new(None)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Virtual microphone initialized (audio output routing)");
        Ok(())
    }

    /// Set the output device by name (e.g., "CABLE Input (VB-Audio Virtual Cable)")
    pub fn set_output_device(&self, device_name: String) {
        info!("Setting audio output device: {}", device_name);
        if let Ok(mut guard) = self.selected_device.lock() {
            *guard = Some(device_name);
        }
    }

    /// Find a cpal output device by name
    fn find_device(&self, name: &str) -> Result<Device> {
        let host = cpal::default_host();
        let devices = host.output_devices()
            .map_err(|e| anyhow!("Failed to enumerate output devices: {}", e))?;

        for device in devices {
            if let Ok(dev_name) = device.name() {
                if dev_name.contains(name) {
                    return Ok(device);
                }
            }
        }

        Err(anyhow!("Output device '{}' not found", name))
    }

    /// Start playing audio file to the selected output device
    pub async fn start_streaming(&self, audio_path: &str) -> Result<()> {
        if self.is_active.load(Ordering::Relaxed) {
            return Err(anyhow!("Already streaming audio"));
        }

        // Validate file
        crate::audio::AudioValidator::validate_audio_file(audio_path)?;

        info!("Starting audio playback: {}", audio_path);

        // Decode the audio file
        let mut decoder = AudioDecoder::new();
        let frames = decoder.decode_all(audio_path, self.audio_config.buffer_size)?;

        if frames.is_empty() {
            return Err(anyhow!("No audio frames decoded from file"));
        }

        info!("Decoded {} audio frames", frames.len());

        // Fill buffer with decoded frames
        {
            let mut buffer = self.audio_buffer.lock()
                .map_err(|_| anyhow!("Failed to lock audio buffer"))?;
            buffer.clear()?;
            for frame in frames {
                buffer.push_frame(frame)?;
            }
        }

        // Get output device
        let device = {
            let selected_name = self.selected_device.lock()
                .map_err(|_| anyhow!("Lock error"))
                .ok()
                .and_then(|g| g.clone());
            match selected_name.as_deref() {
                Some(name) => self.find_device(name)?,
                None => {
                    let host = cpal::default_host();
                    host.default_output_device()
                        .ok_or_else(|| anyhow!("No default output device"))?
                }
            }
        };

        let dev_name = device.name().unwrap_or_default();
        info!("Playing audio to device: {}", dev_name);

        // Get device config
        let supported_config = device.default_output_config()
            .map_err(|e| anyhow!("Failed to get output config: {}", e))?;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        // Set up playback
        self.should_stop.store(false, Ordering::Relaxed);
        let buffer = Arc::clone(&self.audio_buffer);
        let should_stop = Arc::clone(&self.should_stop);

        let stream = match sample_format {
            SampleFormat::F32 => Self::build_stream::<f32>(&device, &config, buffer, should_stop)?,
            SampleFormat::I16 => Self::build_stream::<i16>(&device, &config, buffer, should_stop)?,
            SampleFormat::U16 => Self::build_stream::<u16>(&device, &config, buffer, should_stop)?,
            _ => return Err(anyhow!("Unsupported sample format: {:?}", sample_format)),
        };

        stream.play().map_err(|e| anyhow!("Failed to start playback: {}", e))?;

        *self.audio_stream.lock().map_err(|_| anyhow!("Lock error"))? = Some(stream);
        *self.current_source.lock().map_err(|_| anyhow!("Lock error"))? = Some(audio_path.to_string());
        self.is_active.store(true, Ordering::Relaxed);

        info!("Audio playback started on device: {}", dev_name);
        Ok(())
    }

    fn build_stream<T: cpal::Sample + SizedSample + cpal::FromSample<f32> + Send + 'static>(
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<StdMutex<AudioBuffer>>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<Stream> {
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _| {
                if should_stop.load(Ordering::Relaxed) {
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0f32);
                    }
                    return;
                }

                let mut buf = match buffer.lock() {
                    Ok(b) => b,
                    Err(_) => {
                        for sample in data.iter_mut() {
                            *sample = T::from_sample(0.0f32);
                        }
                        return;
                    }
                };

                if let Some(frame) = buf.pop_frame() {
                    // Convert frame f32 data to output format
                    let f32_samples: &[f32] = unsafe {
                        std::slice::from_raw_parts(
                            frame.data.as_ptr() as *const f32,
                            frame.data.len() / 4,
                        )
                    };

                    for (i, out) in data.iter_mut().enumerate() {
                        if i < f32_samples.len() {
                            *out = T::from_sample(f32_samples[i]);
                        } else {
                            *out = T::from_sample(0.0f32);
                        }
                    }

                    // Re-add frame for looping
                    let _ = buf.push_frame(frame);
                } else {
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0f32);
                    }
                }
            },
            |err| error!("Audio stream error: {}", err),
            None,
        ).map_err(|e| anyhow!("Failed to build output stream: {}", e))?;

        Ok(stream)
    }

    pub async fn stop_streaming(&self) -> Result<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping audio playback");
        self.should_stop.store(true, Ordering::Relaxed);

        // Drop the stream to stop playback
        if let Ok(mut stream) = self.audio_stream.lock() {
            *stream = None;
        }

        // Clear buffer
        if let Ok(mut buf) = self.audio_buffer.lock() {
            let _ = buf.clear();
        }

        if let Ok(mut src) = self.current_source.lock() { *src = None; }
        self.is_active.store(false, Ordering::Relaxed);

        info!("Audio playback stopped");
        Ok(())
    }

    pub async fn set_volume(&self, volume: f32) -> Result<()> {
        let processor = self.audio_processor.lock().await;
        processor.set_volume(volume)
    }

    pub async fn get_volume(&self) -> f32 {
        let processor = self.audio_processor.lock().await;
        processor.get_volume()
    }

    pub async fn set_muted(&self, muted: bool) {
        let processor = self.audio_processor.lock().await;
        processor.set_muted(muted);
    }

    pub async fn is_muted(&self) -> bool {
        let processor = self.audio_processor.lock().await;
        processor.is_muted()
    }

    pub async fn toggle_mute(&self) -> bool {
        let processor = self.audio_processor.lock().await;
        processor.toggle_mute()
    }

    pub async fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    pub fn current_source(&self) -> Option<String> {
        self.current_source.lock().ok().and_then(|g| g.clone())
    }

    pub async fn get_processing_stats(&self) -> AudioProcessorStats {
        let processor = self.audio_processor.lock().await;
        processor.get_stats()
    }

    pub async fn get_visualization_data(&self) -> AudioVisualizationData {
        let visualizer = self.audio_visualizer.lock().await;
        visualizer.get_current_data()
    }

    pub async fn get_buffer_status(&self) -> (usize, usize, u64) {
        let buffer = self.audio_buffer.lock().unwrap();
        (buffer.len(), buffer.max_size(), buffer.total_frames())
    }

    /// List available audio output devices
    pub async fn list_devices() -> Result<Vec<String>> {
        info!("Listing audio output devices");
        let host = cpal::default_host();
        let mut devices = Vec::new();

        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    devices.push(name);
                }
            }
        }

        info!("Found {} audio output devices", devices.len());
        Ok(devices)
    }
}

impl Default for VirtualMicrophone {
    fn default() -> Self {
        Self::new()
    }
}
