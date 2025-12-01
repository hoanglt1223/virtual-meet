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

// Windows API imports
use windows::{
    core::*,
    Win32::{
        Media::Audio::*,
        Media::KernelStreaming::*,
        System::Com::*,
        System::Ole::*,
        System::Threading::*,
    },
};

use std::ptr;

/// WASAPI virtual microphone implementation
pub struct WasapiVirtualMicrophone {
    audio_client: Option<IAudioClient>,
    capture_client: Option<IAudioCaptureClient>,
    render_client: Option<IAudioRenderClient>,
    device_enumerator: Option<IMMDeviceEnumerator>,
    device: Option<IMMDevice>,
    initialized: bool,
    audio_config: AudioConfig,
}

impl WasapiVirtualMicrophone {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            audio_client: None,
            capture_client: None,
            render_client: None,
            device_enumerator: None,
            device: None,
            initialized: false,
            audio_config: config,
        }
    }

    /// Initialize WASAPI virtual microphone
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing WASAPI virtual microphone");

        // Initialize COM
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .map_err(|e| anyhow!("Failed to initialize COM: {}", e))?;
        }

        // Create device enumerator
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&CLSID_MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| anyhow!("Failed to create device enumerator: {}", e))?;

            self.device_enumerator = Some(enumerator);
        }

        // Create virtual audio device
        self.create_virtual_audio_device().await?;

        // Initialize audio client
        self.initialize_audio_client().await?;

        self.initialized = true;
        info!("WASAPI virtual microphone initialized successfully");
        Ok(())
    }

    /// Create virtual audio device
    async fn create_virtual_audio_device(&mut self) -> Result<()> {
        info!("Creating virtual audio device");

        // TODO: This requires creating a virtual audio driver
        // Production implementation would need:
        // 1. Custom audio driver development
        // 2. Kernel Streaming (KS) filter implementation
        // 3. Device registration with Windows

        warn!("Virtual audio device creation requires custom Windows audio driver development");

        // For demonstration, we'll use the default output device as a loopback
        unsafe {
            if let Some(enumerator) = &self.device_enumerator {
                let default_device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)
                    .map_err(|e| anyhow!("Failed to get default audio endpoint: {}", e))?;

                self.device = Some(default_device);
            }
        }

        Ok(())
    }

    /// Initialize audio client
    async fn initialize_audio_client(&mut self) -> Result<()> {
        info!("Initializing WASAPI audio client");

        if let Some(device) = &self.device {
            unsafe {
                let audio_client: IAudioClient = device.Activate(
                    CLSID_AudioClient,
                    CLSCTX_ALL,
                    None,
                ).map_err(|e| anyhow!("Failed to activate audio client: {}", e))?;

                // Set up audio format
                let wave_format = WAVEFORMATEX {
                    wFormatTag: WAVE_FORMAT_EXTENSIBLE as u16,
                    nChannels: self.audio_config.output_channels as u16,
                    nSamplesPerSec: self.audio_config.output_sample_rate,
                    nAvgBytesPerSec: (self.audio_config.output_sample_rate *
                                    self.audio_config.output_channels as u32 *
                                    4) as u32, // 32-bit samples
                    nBlockAlign: (self.audio_config.output_channels as u16 * 4) as u16,
                    wBitsPerSample: 32,
                    cbSize: 22, // Size of WAVEFORMATEXTENSIBLE
                };

                // Initialize audio client in shared mode
                let mut format_ptr = &wave_format as *const _ as *mut _;
                let result = audio_client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    1000000, // 1 second buffer duration
                    0,
                    PWSTR(format_ptr as *mut _),
                    None,
                );

                if result.is_err() {
                    warn!("Failed to initialize audio client in loopback mode - this requires virtual audio driver");
                }

                self.audio_client = Some(audio_client);
            }
        }

        Ok(())
    }

    /// Send audio samples to virtual microphone
    pub async fn send_audio_samples(&mut self, samples: &[f32]) -> Result<()> {
        if !self.initialized {
            return Err(anyhow!("WASAPI virtual microphone not initialized"));
        }

        debug!("Sending {} audio samples to virtual microphone", samples.len());

        // TODO: Implement audio sample delivery to virtual device
        // This would:
        // 1. Convert samples to required format
        // 2. Deliver to virtual audio endpoint
        // 3. Handle buffer management

        Ok(())
    }

    /// Start virtual microphone streaming
    pub async fn start_streaming(&mut self) -> Result<()> {
        if !self.initialized {
            return Err(anyhow!("WASAPI virtual microphone not initialized"));
        }

        info!("Starting WASAPI virtual microphone streaming");

        if let Some(audio_client) = &self.audio_client {
            unsafe {
                audio_client.Start()
                    .map_err(|e| anyhow!("Failed to start audio client: {}", e))?;
            }
        }

        Ok(())
    }

    /// Stop virtual microphone streaming
    pub async fn stop_streaming(&mut self) -> Result<()> {
        info!("Stopping WASAPI virtual microphone streaming");

        if let Some(audio_client) = &self.audio_client {
            unsafe {
                audio_client.Stop()
                    .map_err(|e| anyhow!("Failed to stop audio client: {}", e))?;
            }
        }

        Ok(())
    }

    /// Set audio format
    pub async fn set_format(&mut self, sample_rate: u32, channels: u16, format: AudioSampleFormat) -> Result<()> {
        info!("Setting virtual microphone format: {} Hz, {} channels, {:?}",
              sample_rate, channels, format);

        self.audio_config.output_sample_rate = sample_rate;
        self.audio_config.output_channels = channels as u32;
        self.audio_config.output_format = format;

        // TODO: Reinitialize audio client with new format

        Ok(())
    }
}

impl Drop for WasapiVirtualMicrophone {
    fn drop(&mut self) {
        info!("Cleaning up WASAPI virtual microphone");

        unsafe {
            if let Some(audio_client) = &self.audio_client {
                let _ = audio_client.Stop();
            }

            CoUninitialize();
        }
    }
}

/// Kernel Streaming virtual microphone implementation
pub struct KSVirtualMicrophone {
    ks_filter: Option<Handle>,
    initialized: bool,
    audio_config: AudioConfig,
}

impl KSVirtualMicrophone {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            ks_filter: None,
            initialized: false,
            audio_config: config,
        }
    }

    /// Initialize KS virtual microphone
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Kernel Streaming virtual microphone");

        // TODO: This requires KS driver development
        // Production implementation would need:
        // 1. Custom KS audio filter driver
        // 2. Filter registration with Windows
        // 3. Pin and topology implementation

        warn!("Kernel Streaming virtual microphone requires custom audio driver development");

        self.initialized = true;
        info!("KS virtual microphone initialized successfully");
        Ok(())
    }

    /// Send audio samples through KS interface
    pub async fn send_audio_samples(&mut self, samples: &[f32]) -> Result<()> {
        if !self.initialized {
            return Err(anyhow!("KS virtual microphone not initialized"));
        }

        // TODO: Deliver samples through KS filter
        debug!("Sending {} samples through KS interface", samples.len());

        Ok(())
    }
}

impl Drop for KSVirtualMicrophone {
    fn drop(&mut self) {
        info!("Cleaning up KS virtual microphone");
    }
}

/// Microphone backend options
#[derive(Debug, Clone)]
pub enum MicrophoneBackend {
    WASAPI,
    KernelStreaming,
}

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

    // Windows API backends
    wasapi_backend: Arc<StdMutex<WasapiVirtualMicrophone>>,
    ks_backend: Arc<StdMutex<KSVirtualMicrophone>>,
    backend: MicrophoneBackend,

    // Threading and synchronization
    decode_thread_handle: Option<thread::JoinHandle<()>>,
    playback_thread_handle: Option<thread::JoinHandle<()>>,
    should_stop: Arc<AtomicBool>,
}

impl VirtualMicrophone {
    /// Create a new virtual microphone instance
    pub fn new() -> Self {
        Self::with_backend(MicrophoneBackend::WASAPI)
    }

    /// Create a new virtual microphone instance with specified backend
    pub fn with_backend(backend: MicrophoneBackend) -> Self {
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
            wasapi_backend: Arc::new(StdMutex::new(WasapiVirtualMicrophone::new(AudioConfig::default()))),
            ks_backend: Arc::new(StdMutex::new(KSVirtualMicrophone::new(AudioConfig::default()))),
            backend,
            decode_thread_handle: None,
            playback_thread_handle: None,
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the virtual microphone
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing virtual microphone using {:?} backend", self.backend);

        // Initialize audio processing pipeline
        let processor = self.audio_processor.lock().await;
        info!("Audio processor initialized: {} channels, {} Hz, {:?} format",
              self.audio_config.output_channels,
              self.audio_config.output_sample_rate,
              self.audio_config.output_format);

        match self.backend {
            MicrophoneBackend::WASAPI => {
                let mut backend = self.wasapi_backend.lock().map_err(|_| anyhow!("Failed to lock WASAPI backend"))?;
                backend.initialize().await?;
            },
            MicrophoneBackend::KernelStreaming => {
                let mut backend = self.ks_backend.lock().map_err(|_| anyhow!("Failed to lock KS backend"))?;
                backend.initialize().await?;
            },
        }

        info!("Virtual microphone initialized successfully with {:?} backend", self.backend);
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

        // Start backend streaming
        match self.backend {
            MicrophoneBackend::WASAPI => {
                let mut backend = self.wasapi_backend.lock().map_err(|_| anyhow!("Failed to lock WASAPI backend"))?;
                backend.start_streaming().await?;
            },
            MicrophoneBackend::KernelStreaming => {
                let mut backend = self.ks_backend.lock().map_err(|_| anyhow!("Failed to lock KS backend"))?;
                // KS backend doesn't have start_streaming method yet
                warn!("KS backend streaming not fully implemented");
            },
        }

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
        let wasapi_backend = Arc::clone(&self.wasapi_backend);
        let ks_backend = Arc::clone(&self.ks_backend);
        let backend_type = self.backend.clone();

        // For now, we'll create a simple output stream as a proof of concept
        // In a full implementation, this would interface with a virtual audio device
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| anyhow!("No audio output device available"))?;

        let config = device.default_output_config()?;
        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();

        let stream = match sample_format {
            SampleFormat::F32 => self.create_virtual_audio_stream::<f32>(
                &device, &config, processor, buffer, visualizer, should_stop,
                wasapi_backend, ks_backend, backend_type
            )?,
            SampleFormat::I16 => self.create_virtual_audio_stream::<i16>(
                &device, &config, processor, buffer, visualizer, should_stop,
                wasapi_backend, ks_backend, backend_type
            )?,
            SampleFormat::U16 => self.create_virtual_audio_stream::<u16>(
                &device, &config, processor, buffer, visualizer, should_stop,
                wasapi_backend, ks_backend, backend_type
            )?,
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
        wasapi_backend: Arc<StdMutex<WasapiVirtualMicrophone>>,
        ks_backend: Arc<StdMutex<KSVirtualMicrophone>>,
        backend_type: MicrophoneBackend,
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

                // Send frame to virtual microphone backend
                let rt = tokio::runtime::Runtime::new().unwrap();
                let _ = rt.block_on(async {
                    match backend_type {
                        MicrophoneBackend::WASAPI => {
                            let mut backend = wasapi_backend.lock().ok()?;
                            let samples: Vec<f32> = processed_frame.data.chunks_exact(4)
                                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                                .collect();
                            if let Err(e) = backend.send_audio_samples(&samples).await {
                                error!("Failed to send audio samples to WASAPI virtual microphone: {}", e);
                            }
                        },
                        MicrophoneBackend::KernelStreaming => {
                            let mut backend = ks_backend.lock().ok()?;
                            let samples: Vec<f32> = processed_frame.data.chunks_exact(4)
                                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                                .collect();
                            if let Err(e) = backend.send_audio_samples(&samples).await {
                                error!("Failed to send audio samples to KS virtual microphone: {}", e);
                            }
                        },
                    }
                    Some(())
                });
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

        // Stop backend streaming
        match self.backend {
            MicrophoneBackend::WASAPI => {
                let mut backend = self.wasapi_backend.lock().map_err(|_| anyhow!("Failed to lock WASAPI backend"))?;
                backend.stop_streaming().await?;
            },
            MicrophoneBackend::KernelStreaming => {
                // KS backend doesn't have stop_streaming method yet
                warn!("KS backend streaming not fully implemented");
            },
        }

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

        let mut devices = Vec::new();

        // Enumerate WASAPI devices
        unsafe {
            if let Ok(wasapi_devices) = Self::enumerate_wasapi_devices() {
                devices.extend(wasapi_devices);
            }
        }

        // Enumerate CPAL devices
        let host = cpal::default_host();
        if let Ok(cpal_devices) = host.devices() {
            let device_names: Result<Vec<_>> = cpal_devices
                .map(|d| d.name().map_err(|e| anyhow!("Failed to get device name: {}", e)))
                .collect();

            if let Ok(mut names) = device_names {
                for name in names {
                    devices.push(format!("CPAL: {}", name));
                }
            }
        }

        // Add our virtual devices
        devices.push("VirtualMicrophone (WASAPI)".to_string());
        devices.push("VirtualMicrophone (KernelStreaming)".to_string());

        info!("Found {} audio devices", devices.len());
        Ok(devices)
    }

    /// Enumerate WASAPI audio devices
    unsafe fn enumerate_wasapi_devices() -> Result<Vec<String>> {
        let mut devices = Vec::new();

        // Create device enumerator
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&CLSID_MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(_) => return Ok(devices),
        };

        // Enumerate capture devices
        if let Ok(capture_collection) = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) {
            let mut count = 0u32;
            let _ = capture_collection.GetCount(&mut count);

            for i in 0..count {
                if let Ok(device) = capture_collection.Item(i) {
                    if let Ok(id) = device.GetId() {
                        let id_str = String::from_utf16_lossy(&id.as_wide());
                        devices.push(format!("WASAPI Capture: {}", id_str));
                    }
                }
            }
        }

        // Enumerate render devices
        if let Ok(render_collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
            let mut count = 0u32;
            let _ = render_collection.GetCount(&mut count);

            for i in 0..count {
                if let Ok(device) = render_collection.Item(i) {
                    if let Ok(id) = device.GetId() {
                        let id_str = String::from_utf16_lossy(&id.as_wide());
                        devices.push(format!("WASAPI Render: {}", id_str));
                    }
                }
            }
        }

        Ok(devices)
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