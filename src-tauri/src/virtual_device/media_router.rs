//! Media Router - coordinates video and audio streaming

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use super::{VirtualMicrophone, VirtualWebcam};

/// Media routing configuration
#[derive(Debug, Clone, Serialize)]
pub struct MediaRoutingConfig {
    pub video_path: String,
    pub audio_path: String,
    pub video_device: Option<String>,
    pub audio_device: Option<String>,
    pub loop_media: bool,
    pub video_volume: f32,
    pub audio_volume: f32,
}

impl Default for MediaRoutingConfig {
    fn default() -> Self {
        Self {
            video_path: String::new(),
            audio_path: String::new(),
            video_device: None,
            audio_device: None,
            loop_media: true,
            video_volume: 1.0,
            audio_volume: 1.0,
        }
    }
}

/// Media routing status
#[derive(Debug, Clone, Serialize)]
pub struct MediaRoutingStatus {
    pub is_active: bool,
    pub video_active: bool,
    pub audio_active: bool,
    pub config: MediaRoutingConfig,
}

/// Media router coordinates webcam and microphone streaming
pub struct MediaRouter {
    webcam: Arc<VirtualWebcam>,
    microphone: Arc<Mutex<VirtualMicrophone>>,
    is_active: Arc<AtomicBool>,
    config: Arc<Mutex<MediaRoutingConfig>>,
}

impl MediaRouter {
    pub fn new() -> Self {
        Self::with_config(MediaRoutingConfig::default())
    }

    pub fn with_config(config: MediaRoutingConfig) -> Self {
        Self {
            webcam: Arc::new(VirtualWebcam::new()),
            microphone: Arc::new(Mutex::new(VirtualMicrophone::new())),
            is_active: Arc::new(AtomicBool::new(false)),
            config: Arc::new(Mutex::new(config)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        self.webcam.initialize().await?;
        let mic = self.microphone.lock().await;
        mic.initialize().await?;
        info!("Media router initialized");
        Ok(())
    }

    pub async fn start(&self, config: MediaRoutingConfig) -> Result<()> {
        info!("Starting media routing");

        // Set device targets
        if let Some(ref video_device) = config.video_device {
            self.webcam.set_target_device(video_device.clone()).await;
        }
        if let Some(ref audio_device) = config.audio_device {
            let mic = self.microphone.lock().await;
            mic.set_output_device(audio_device.clone());
        }

        // Start video if path provided
        if !config.video_path.is_empty() {
            self.webcam.start_streaming(&config.video_path).await?;
        }

        // Start audio if path provided
        if !config.audio_path.is_empty() {
            let mic = self.microphone.lock().await;
            mic.start_streaming(&config.audio_path).await?;
        }

        *self.config.lock().await = config;
        self.is_active.store(true, Ordering::Relaxed);
        info!("Media routing started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping media routing");
        let _ = self.webcam.stop_streaming().await;
        let mic = self.microphone.lock().await;
        let _ = mic.stop_streaming().await;
        self.is_active.store(false, Ordering::Relaxed);
        info!("Media routing stopped");
        Ok(())
    }

    pub async fn switch_media(
        &self,
        video_path: Option<String>,
        audio_path: Option<String>,
    ) -> Result<()> {
        if let Some(path) = video_path {
            let _ = self.webcam.stop_streaming().await;
            if !path.is_empty() {
                self.webcam.start_streaming(&path).await?;
            }
        }
        if let Some(path) = audio_path {
            let mic = self.microphone.lock().await;
            let _ = mic.stop_streaming().await;
            if !path.is_empty() {
                mic.start_streaming(&path).await?;
            }
        }
        Ok(())
    }

    pub async fn get_status(&self) -> MediaRoutingStatus {
        MediaRoutingStatus {
            is_active: self.is_active.load(Ordering::Relaxed),
            video_active: self.webcam.is_active().await,
            audio_active: self.microphone.lock().await.is_active().await,
            config: self.config.lock().await.clone(),
        }
    }
}

impl Default for MediaRouter {
    fn default() -> Self {
        Self::new()
    }
}
