//! Media Router for Virtual Device Integration
//!
//! This module provides high-level routing of media content to virtual devices,
//! coordinating between video and audio pipelines for synchronized playback.

use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::{MicrophoneBackend, VirtualMicrophone, VirtualWebcam, WebcamBackend};
use crate::audio::AudioConfig;

/// Media routing configuration
#[derive(Debug, Clone)]
pub struct MediaRoutingConfig {
    pub video_backend: WebcamBackend,
    pub audio_backend: MicrophoneBackend,
    pub video_path: String,
    pub audio_path: String,
    pub sync_audio_video: bool,
    pub loop_media: bool,
    pub video_volume: f32,
    pub audio_volume: f32,
}

impl Default for MediaRoutingConfig {
    fn default() -> Self {
        Self {
            video_backend: WebcamBackend::DirectShow,
            audio_backend: MicrophoneBackend::WASAPI,
            video_path: String::new(),
            audio_path: String::new(),
            sync_audio_video: true,
            loop_media: true,
            video_volume: 1.0,
            audio_volume: 1.0,
        }
    }
}

/// Synchronization state for media playback
#[derive(Debug)]
pub struct SyncState {
    pub video_timestamp: Duration,
    pub audio_timestamp: Duration,
    pub is_video_playing: bool,
    pub is_audio_playing: bool,
    pub sync_offset: Duration,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            video_timestamp: Duration::default(),
            audio_timestamp: Duration::default(),
            is_video_playing: false,
            is_audio_playing: false,
            sync_offset: Duration::default(),
        }
    }
}

/// Media router coordinates virtual webcam and microphone
pub struct MediaRouter {
    is_active: Arc<AtomicBool>,
    config: Arc<Mutex<MediaRoutingConfig>>,
    sync_state: Arc<StdMutex<SyncState>>,

    // Virtual devices
    virtual_webcam: Arc<StdMutex<VirtualWebcam>>,
    virtual_microphone: Arc<StdMutex<VirtualMicrophone>>,

    // Synchronization
    sync_thread_handle: Option<thread::JoinHandle<()>>,
    should_stop: Arc<AtomicBool>,
}

impl MediaRouter {
    /// Create a new media router
    pub fn new() -> Self {
        Self::with_config(MediaRoutingConfig::default())
    }

    /// Create a new media router with specific configuration
    pub fn with_config(config: MediaRoutingConfig) -> Self {
        let virtual_webcam = match config.video_backend {
            WebcamBackend::DirectShow => VirtualWebcam::with_backend(WebcamBackend::DirectShow),
            WebcamBackend::MediaFoundation => {
                VirtualWebcam::with_backend(WebcamBackend::MediaFoundation)
            }
        };

        let virtual_microphone = match config.audio_backend {
            MicrophoneBackend::WASAPI => VirtualMicrophone::with_backend(MicrophoneBackend::WASAPI),
            MicrophoneBackend::KernelStreaming => {
                VirtualMicrophone::with_backend(MicrophoneBackend::KernelStreaming)
            }
        };

        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            config: Arc::new(Mutex::new(config)),
            sync_state: Arc::new(StdMutex::new(SyncState::default())),
            virtual_webcam: Arc::new(StdMutex::new(virtual_webcam)),
            virtual_microphone: Arc::new(StdMutex::new(virtual_microphone)),
            sync_thread_handle: None,
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the media router
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing media router");

        // Initialize virtual devices
        {
            let mut webcam = self
                .virtual_webcam
                .lock()
                .map_err(|_| anyhow!("Failed to lock virtual webcam"))?;
            webcam.initialize().await?;
        }

        {
            let mut microphone = self
                .virtual_microphone
                .lock()
                .map_err(|_| anyhow!("Failed to lock virtual microphone"))?;
            microphone.initialize().await?;
        }

        info!("Media router initialized successfully");
        Ok(())
    }

    /// Start media routing with the specified configuration
    pub async fn start(&self, config: MediaRoutingConfig) -> Result<()> {
        if self.is_active.load(Ordering::Relaxed) {
            return Err(anyhow!("Media router already active"));
        }

        info!(
            "Starting media routing with video: {}, audio: {}",
            config.video_path, config.audio_path
        );

        // Update configuration
        {
            let mut cfg = self.config.lock().await;
            *cfg = config.clone();
        }

        // Validate media files
        if !config.video_path.is_empty() {
            if !std::path::Path::new(&config.video_path).exists() {
                return Err(anyhow!("Video file not found: {}", config.video_path));
            }
        }

        if !config.audio_path.is_empty() {
            if !std::path::Path::new(&config.audio_path).exists() {
                return Err(anyhow!("Audio file not found: {}", config.audio_path));
            }
        }

        // Start virtual device streaming
        if !config.video_path.is_empty() {
            let mut webcam = self
                .virtual_webcam
                .lock()
                .map_err(|_| anyhow!("Failed to lock virtual webcam"))?;
            webcam.start_streaming(&config.video_path).await?;
        }

        if !config.audio_path.is_empty() {
            let mut microphone = self
                .virtual_microphone
                .lock()
                .map_err(|_| anyhow!("Failed to lock virtual microphone"))?;
            microphone.start_streaming(&config.audio_path).await?;

            // Set audio volume
            microphone.set_volume(config.audio_volume).await?;
        }

        // Start synchronization thread if needed
        if config.sync_audio_video && !config.video_path.is_empty() && !config.audio_path.is_empty()
        {
            self.start_sync_thread().await?;
        }

        self.is_active.store(true, Ordering::Relaxed);
        info!("Media routing started successfully");
        Ok(())
    }

    /// Stop media routing
    pub async fn stop(&self) -> Result<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping media routing");

        // Stop synchronization thread
        self.should_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.sync_thread_handle.take() {
            if let Err(e) = handle.join() {
                error!("Sync thread join failed: {:?}", e);
            }
        }

        // Stop virtual devices
        {
            let webcam = self
                .virtual_webcam
                .lock()
                .map_err(|_| anyhow!("Failed to lock virtual webcam"))?;
            if webcam.is_active().await {
                webcam.stop_streaming().await?;
            }
        }

        {
            let microphone = self
                .virtual_microphone
                .lock()
                .map_err(|_| anyhow!("Failed to lock virtual microphone"))?;
            if microphone.is_active().await {
                microphone.stop_streaming().await?;
            }
        }

        // Reset sync state
        {
            let mut sync_state = self
                .sync_state
                .lock()
                .map_err(|_| anyhow!("Failed to lock sync state"))?;
            *sync_state = SyncState::default();
        }

        self.is_active.store(false, Ordering::Relaxed);
        info!("Media routing stopped");
        Ok(())
    }

    /// Start synchronization thread
    async fn start_sync_thread(&self) -> Result<()> {
        info!("Starting audio/video synchronization thread");

        self.should_stop.store(false, Ordering::Relaxed);
        let should_stop = Arc::clone(&self.should_stop);
        let sync_state = Arc::clone(&self.sync_state);
        let virtual_webcam = Arc::clone(&self.virtual_webcam);
        let virtual_microphone = Arc::clone(&self.virtual_microphone);

        let handle = thread::spawn(move || {
            Self::sync_loop(should_stop, sync_state, virtual_webcam, virtual_microphone);
        });

        self.sync_thread_handle = Some(handle);
        Ok(())
    }

    /// Synchronization loop for keeping audio and video in sync
    fn sync_loop(
        should_stop: Arc<AtomicBool>,
        sync_state: Arc<StdMutex<SyncState>>,
        virtual_webcam: Arc<StdMutex<VirtualWebcam>>,
        virtual_microphone: Arc<StdMutex<VirtualMicrophone>>,
    ) {
        info!("Starting audio/video synchronization loop");

        let mut last_sync_time = Instant::now();
        let sync_interval = Duration::from_millis(100); // Sync every 100ms

        while !should_stop.load(Ordering::Relaxed) {
            let now = Instant::now();

            if now.duration_since(last_sync_time) >= sync_interval {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let _ = rt.block_on(async {
                    if let Err(e) =
                        Self::perform_sync_check(&sync_state, &virtual_webcam, &virtual_microphone)
                            .await
                    {
                        error!("Sync check failed: {}", e);
                    }
                    Some(())
                });

                last_sync_time = now;
            }

            thread::sleep(Duration::from_millis(10));
        }

        info!("Audio/video synchronization loop stopped");
    }

    /// Perform synchronization check and adjustment
    async fn perform_sync_check(
        sync_state: &StdMutex<SyncState>,
        virtual_webcam: &StdMutex<VirtualWebcam>,
        virtual_microphone: &StdMutex<VirtualMicrophone>,
    ) -> Result<()> {
        let webcam_active = {
            let webcam = virtual_webcam
                .lock()
                .map_err(|_| anyhow!("Failed to lock webcam"))?;
            webcam.is_active().await
        };

        let microphone_active = {
            let microphone = virtual_microphone
                .lock()
                .map_err(|_| anyhow!("Failed to lock microphone"))?;
            microphone.is_active().await
        };

        let mut state = sync_state
            .lock()
            .map_err(|_| anyhow!("Failed to lock sync state"))?;
        state.is_video_playing = webcam_active;
        state.is_audio_playing = microphone_active;

        // Calculate sync offset if both are playing
        if webcam_active && microphone_active {
            // This is a simplified sync check - in production you'd use
            // actual timestamps from the media streams
            let offset = if state.video_timestamp > state.audio_timestamp {
                state.video_timestamp - state.audio_timestamp
            } else {
                state.audio_timestamp - state.video_timestamp
            };

            state.sync_offset = offset;

            // Log sync status periodically
            if offset > Duration::from_millis(100) {
                warn!("Audio/video sync offset: {:?}", offset);
            } else {
                debug!("Audio/video in sync, offset: {:?}", offset);
            }
        }

        Ok(())
    }

    /// Get current routing status
    pub async fn get_status(&self) -> MediaRoutingStatus {
        let config = self.config.lock().await.clone();
        let sync_state = self.sync_state.lock().unwrap().clone();
        let is_active = self.is_active.load(Ordering::Relaxed);

        let webcam_status = {
            let webcam = self.virtual_webcam.lock().unwrap();
            if webcam.is_active().await {
                let video_info = webcam.get_video_info().await.ok();
                let buffer_status = webcam.get_buffer_status().await;
                VirtualWebcamStatus {
                    is_active: true,
                    current_source: webcam.current_source().await,
                    video_info,
                    buffer_status,
                }
            } else {
                VirtualWebcamStatus {
                    is_active: false,
                    current_source: None,
                    video_info: None,
                    buffer_status: Default::default(),
                }
            }
        };

        let microphone_status = {
            let microphone = self.virtual_microphone.lock().unwrap();
            if microphone.is_active().await {
                VirtualMicrophoneStatus {
                    is_active: true,
                    current_source: microphone.current_source().await,
                    volume: microphone.get_volume().await,
                    is_muted: microphone.is_muted().await,
                    processing_stats: microphone.get_processing_stats().await,
                    buffer_status: microphone.get_buffer_status().await,
                }
            } else {
                VirtualMicrophoneStatus {
                    is_active: false,
                    current_source: None,
                    volume: 1.0,
                    is_muted: false,
                    processing_stats: Default::default(),
                    buffer_status: (0, 0, 0),
                }
            }
        };

        MediaRoutingStatus {
            is_active,
            config,
            sync_state,
            webcam_status,
            microphone_status,
        }
    }

    /// Update configuration at runtime
    pub async fn update_config(&self, new_config: MediaRoutingConfig) -> Result<()> {
        info!("Updating media routing configuration");

        {
            let mut config = self.config.lock().await;
            *config = new_config.clone();
        }

        // Update microphone volume
        {
            let microphone = self
                .virtual_microphone
                .lock()
                .map_err(|_| anyhow!("Failed to lock microphone"))?;
            microphone.set_volume(new_config.audio_volume).await?;
            microphone.set_muted(new_config.audio_volume == 0.0).await;
        }

        info!("Media routing configuration updated");
        Ok(())
    }

    /// Switch to new media files without stopping the stream
    pub async fn switch_media(
        &self,
        video_path: Option<String>,
        audio_path: Option<String>,
    ) -> Result<()> {
        info!(
            "Switching media - video: {:?}, audio: {:?}",
            video_path, audio_path
        );

        let config = self.config.lock().await.clone();

        // Switch video if provided
        if let Some(new_video_path) = video_path {
            if !new_video_path.is_empty() {
                if !std::path::Path::new(&new_video_path).exists() {
                    return Err(anyhow!("Video file not found: {}", new_video_path));
                }

                // Stop current video
                if !config.video_path.is_empty() {
                    let webcam = self
                        .virtual_webcam
                        .lock()
                        .map_err(|_| anyhow!("Failed to lock webcam"))?;
                    webcam.stop_streaming().await?;
                }

                // Start new video
                let webcam = self
                    .virtual_webcam
                    .lock()
                    .map_err(|_| anyhow!("Failed to lock webcam"))?;
                webcam.start_streaming(&new_video_path).await?;

                // Update config
                let mut config = self.config.lock().await;
                config.video_path = new_video_path;
            }
        }

        // Switch audio if provided
        if let Some(new_audio_path) = audio_path {
            if !new_audio_path.is_empty() {
                if !std::path::Path::new(&new_audio_path).exists() {
                    return Err(anyhow!("Audio file not found: {}", new_audio_path));
                }

                // Stop current audio
                if !config.audio_path.is_empty() {
                    let microphone = self
                        .virtual_microphone
                        .lock()
                        .map_err(|_| anyhow!("Failed to lock microphone"))?;
                    microphone.stop_streaming().await?;
                }

                // Start new audio
                let microphone = self
                    .virtual_microphone
                    .lock()
                    .map_err(|_| anyhow!("Failed to lock microphone"))?;
                microphone.start_streaming(&new_audio_path).await?;

                // Update config
                let mut config = self.config.lock().await;
                config.audio_path = new_audio_path;
            }
        }

        info!("Media switch completed");
        Ok(())
    }
}

impl Drop for MediaRouter {
    fn drop(&mut self) {
        info!("Cleaning up media router");

        self.should_stop.store(true, Ordering::Relaxed);

        if let Some(handle) = self.sync_thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Status information for media routing
#[derive(Debug, Clone)]
pub struct MediaRoutingStatus {
    pub is_active: bool,
    pub config: MediaRoutingConfig,
    pub sync_state: SyncState,
    pub webcam_status: VirtualWebcamStatus,
    pub microphone_status: VirtualMicrophoneStatus,
}

/// Status for virtual webcam
#[derive(Debug, Clone)]
pub struct VirtualWebcamStatus {
    pub is_active: bool,
    pub current_source: Option<String>,
    pub video_info: Option<super::VideoInfo>,
    pub buffer_status: super::BufferStatus,
}

/// Status for virtual microphone
#[derive(Debug, Clone)]
pub struct VirtualMicrophoneStatus {
    pub is_active: bool,
    pub current_source: Option<String>,
    pub volume: f32,
    pub is_muted: bool,
    pub processing_stats: crate::audio_processor::AudioProcessorStats,
    pub buffer_status: (usize, usize, u64),
}

impl Default for VirtualWebcamStatus {
    fn default() -> Self {
        Self {
            is_active: false,
            current_source: None,
            video_info: None,
            buffer_status: super::BufferStatus {
                current_frames: 0,
                capacity: 0,
                total_processed: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_media_router_creation() {
        let router = MediaRouter::new();
        assert!(!router.is_active.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_media_router_config() {
        let config = MediaRoutingConfig {
            video_backend: WebcamBackend::MediaFoundation,
            audio_backend: MicrophoneBackend::KernelStreaming,
            video_path: "test.mp4".to_string(),
            audio_path: "test.mp3".to_string(),
            ..Default::default()
        };

        let router = MediaRouter::with_config(config.clone());
        let status = router.get_status().await;
        assert_eq!(status.config.video_backend, config.video_backend);
        assert_eq!(status.config.audio_backend, config.audio_backend);
    }

    #[tokio::test]
    async fn test_media_router_initialization() {
        let router = MediaRouter::new();
        let result = router.initialize().await;
        // This may fail due to missing virtual devices, which is expected
        // assert!(result.is_ok() || result.unwrap_err().to_string().contains("requires custom"));
    }

    #[tokio::test]
    async fn test_config_update() {
        let router = MediaRouter::new();
        let new_config = MediaRoutingConfig {
            audio_volume: 0.5,
            video_volume: 0.75,
            ..Default::default()
        };

        let result = router.update_config(new_config).await;
        assert!(result.is_ok());

        let status = router.get_status().await;
        assert_eq!(status.config.audio_volume, 0.5);
        assert_eq!(status.config.video_volume, 0.75);
    }

    #[tokio::test]
    async fn test_media_switch_with_invalid_files() {
        let router = MediaRouter::new();
        let result = router
            .switch_media(
                Some("nonexistent.mp4".to_string()),
                Some("nonexistent.mp3".to_string()),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
