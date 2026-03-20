//! Tauri Commands for Virtual Device Management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::virtual_device::{
    MediaRouter, MediaRoutingConfig, VirtualMicrophone, VirtualWebcam,
};

/// Shared state for virtual devices
pub struct VirtualDeviceState {
    pub webcam: Arc<Mutex<Option<VirtualWebcam>>>,
    pub microphone: Arc<Mutex<Option<VirtualMicrophone>>>,
    pub media_router: Arc<Mutex<Option<MediaRouter>>>,
}

impl Default for VirtualDeviceState {
    fn default() -> Self {
        Self {
            webcam: Arc::new(Mutex::new(None)),
            microphone: Arc::new(Mutex::new(None)),
            media_router: Arc::new(Mutex::new(None)),
        }
    }
}

/// Virtual device status information
#[derive(Debug, Serialize, Clone)]
pub struct VirtualDeviceStatus {
    pub webcam_active: bool,
    pub microphone_active: bool,
    pub media_router_active: bool,
    pub webcam_backend: Option<String>,
    pub microphone_backend: Option<String>,
}

/// Initialize virtual webcam device
#[command]
pub async fn initialize_webcam(state: State<'_, VirtualDeviceState>) -> Result<(), String> {
    info!("Initializing virtual webcam");

    let webcam = VirtualWebcam::new();

    if let Err(e) = webcam.initialize().await {
        error!("Failed to initialize virtual webcam: {}", e);
        return Err(format!("Failed to initialize virtual webcam: {}", e));
    }

    *state.webcam.lock().await = Some(webcam);
    info!("Virtual webcam initialized successfully");
    Ok(())
}

/// Initialize virtual microphone device
#[command]
pub async fn initialize_microphone(state: State<'_, VirtualDeviceState>) -> Result<(), String> {
    info!("Initializing virtual microphone");

    let microphone = VirtualMicrophone::new();

    if let Err(e) = microphone.initialize().await {
        error!("Failed to initialize virtual microphone: {}", e);
        return Err(format!("Failed to initialize virtual microphone: {}", e));
    }

    *state.microphone.lock().await = Some(microphone);
    info!("Virtual microphone initialized successfully");
    Ok(())
}

/// Start virtual webcam streaming
#[command]
pub async fn start_webcam_streaming(
    video_path: String,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Starting webcam streaming for: {}", video_path);

    let webcam_lock = state.webcam.lock().await;
    let webcam = webcam_lock.as_ref().ok_or("Virtual webcam not initialized")?;

    webcam.start_streaming(&video_path).await
        .map_err(|e| format!("Failed to start webcam streaming: {}", e))?;

    info!("Webcam streaming started successfully");
    Ok(())
}

/// Start virtual microphone streaming
#[command]
pub async fn start_microphone_streaming(
    audio_path: String,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Starting microphone streaming for: {}", audio_path);

    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;

    microphone.start_streaming(&audio_path).await
        .map_err(|e| format!("Failed to start microphone streaming: {}", e))?;

    info!("Microphone streaming started successfully");
    Ok(())
}

/// Stop virtual webcam streaming
#[command]
pub async fn stop_webcam_streaming(state: State<'_, VirtualDeviceState>) -> Result<(), String> {
    info!("Stopping webcam streaming");

    let webcam_lock = state.webcam.lock().await;
    let webcam = webcam_lock.as_ref().ok_or("Virtual webcam not initialized")?;

    webcam.stop_streaming().await
        .map_err(|e| format!("Failed to stop webcam streaming: {}", e))?;

    info!("Webcam streaming stopped successfully");
    Ok(())
}

/// Stop virtual microphone streaming
#[command]
pub async fn stop_microphone_streaming(
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Stopping microphone streaming");

    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;

    microphone.stop_streaming().await
        .map_err(|e| format!("Failed to stop microphone streaming: {}", e))?;

    info!("Microphone streaming stopped successfully");
    Ok(())
}

/// Get virtual device status
#[command]
pub async fn get_virtual_device_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<VirtualDeviceStatus, String> {
    let webcam_active = match state.webcam.lock().await.as_ref() {
        Some(w) => w.is_active().await,
        None => false,
    };

    let microphone_active = match state.microphone.lock().await.as_ref() {
        Some(m) => m.is_active().await,
        None => false,
    };

    let media_router_active = match state.media_router.lock().await.as_ref() {
        Some(r) => r.get_status().await.is_active,
        None => false,
    };

    let webcam_backend = state.webcam.lock().await.as_ref().map(|_| "ffmpeg".to_string());
    let microphone_backend = state.microphone.lock().await.as_ref().map(|_| "cpal".to_string());

    Ok(VirtualDeviceStatus {
        webcam_active,
        microphone_active,
        media_router_active,
        webcam_backend,
        microphone_backend,
    })
}

/// List available virtual devices
#[command]
pub async fn list_virtual_devices() -> Result<Vec<String>, String> {
    info!("Listing virtual devices");
    let mut devices = Vec::new();

    match VirtualWebcam::list_devices().await {
        Ok(webcam_devices) => {
            devices.push("Webcam Devices:".to_string());
            devices.extend(webcam_devices);
        }
        Err(e) => warn!("Failed to enumerate webcam devices: {}", e),
    }

    match VirtualMicrophone::list_devices().await {
        Ok(mic_devices) => {
            devices.push("Microphone Devices:".to_string());
            devices.extend(mic_devices);
        }
        Err(e) => warn!("Failed to enumerate microphone devices: {}", e),
    }

    Ok(devices)
}

/// Initialize media router
#[command]
pub async fn initialize_media_router(
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Initializing media router");

    let media_router = MediaRouter::new();

    if let Err(e) = media_router.initialize().await {
        error!("Failed to initialize media router: {}", e);
        return Err(format!("Failed to initialize media router: {}", e));
    }

    *state.media_router.lock().await = Some(media_router);
    info!("Media router initialized successfully");
    Ok(())
}

/// Start media routing
#[command]
pub async fn start_media_routing(
    video_path: Option<String>,
    audio_path: Option<String>,
    loop_media: bool,
    video_volume: f32,
    audio_volume: f32,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Starting media routing");

    let router_lock = state.media_router.lock().await;
    let media_router = router_lock.as_ref().ok_or("Media router not initialized")?;

    let config = MediaRoutingConfig {
        video_path: video_path.unwrap_or_default(),
        audio_path: audio_path.unwrap_or_default(),
        loop_media,
        video_volume,
        audio_volume,
        ..Default::default()
    };

    media_router.start(config).await
        .map_err(|e| format!("Failed to start media routing: {}", e))?;

    info!("Media routing started successfully");
    Ok(())
}

/// Stop media routing
#[command]
pub async fn stop_media_routing(state: State<'_, VirtualDeviceState>) -> Result<(), String> {
    info!("Stopping media routing");

    let router_lock = state.media_router.lock().await;
    let media_router = router_lock.as_ref().ok_or("Media router not initialized")?;

    media_router.stop().await
        .map_err(|e| format!("Failed to stop media routing: {}", e))?;

    info!("Media routing stopped successfully");
    Ok(())
}

/// Switch media files
#[command]
pub async fn switch_media(
    video_path: Option<String>,
    audio_path: Option<String>,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    let router_lock = state.media_router.lock().await;
    let media_router = router_lock.as_ref().ok_or("Media router not initialized")?;

    media_router.switch_media(video_path, audio_path).await
        .map_err(|e| format!("Failed to switch media: {}", e))
}

/// Set microphone volume
#[command]
pub async fn set_vd_microphone_volume(
    volume: f32,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;

    microphone.set_volume(volume).await
        .map_err(|e| format!("Failed to set microphone volume: {}", e))
}

/// Get microphone volume
#[command]
pub async fn get_microphone_volume(
    state: State<'_, VirtualDeviceState>,
) -> Result<f32, String> {
    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;
    Ok(microphone.get_volume().await)
}

/// Set microphone mute state
#[command]
pub async fn set_vd_microphone_muted(
    muted: bool,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;
    microphone.set_muted(muted).await;
    Ok(())
}

/// Get microphone mute state
#[command]
pub async fn get_microphone_muted(
    state: State<'_, VirtualDeviceState>,
) -> Result<bool, String> {
    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;
    Ok(microphone.is_muted().await)
}

/// Get detailed media routing status
#[command]
pub async fn get_media_routing_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<crate::virtual_device::MediaRoutingStatus, String> {
    let router_lock = state.media_router.lock().await;
    let media_router = router_lock.as_ref().ok_or("Media router not initialized")?;
    Ok(media_router.get_status().await)
}

/// Get webcam video information
#[command]
pub async fn get_webcam_video_info(
    state: State<'_, VirtualDeviceState>,
) -> Result<Option<crate::virtual_device::VideoInfo>, String> {
    let webcam_lock = state.webcam.lock().await;
    let webcam = webcam_lock.as_ref().ok_or("Virtual webcam not initialized")?;

    webcam.get_video_info().await
        .map_err(|e| format!("Failed to get video info: {}", e))
}

/// Get webcam buffer status
#[command]
pub async fn get_webcam_buffer_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<crate::virtual_device::BufferStatus, String> {
    let webcam_lock = state.webcam.lock().await;
    let webcam = webcam_lock.as_ref().ok_or("Virtual webcam not initialized")?;
    Ok(webcam.get_buffer_status().await)
}

/// Get microphone buffer status
#[command]
pub async fn get_microphone_buffer_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<(usize, usize, u64), String> {
    let mic_lock = state.microphone.lock().await;
    let microphone = mic_lock.as_ref().ok_or("Virtual microphone not initialized")?;
    Ok(microphone.get_buffer_status().await)
}
