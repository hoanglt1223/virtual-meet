//! Tauri Commands for Virtual Device Management
//!
//! This module provides Tauri commands for controlling virtual webcam and microphone devices.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::{command, State};
use std::sync::{Arc, Mutex as StdMutex};
use tracing::{info, error, warn, debug};

use crate::virtual::{VirtualWebcam, WebcamBackend, VirtualMicrophone, MicrophoneBackend, MediaRouter, MediaRoutingConfig};
use crate::audio::AudioConfig;

/// Shared state for virtual devices
pub struct VirtualDeviceState {
    pub webcam: Arc<StdMutex<Option<VirtualWebcam>>>,
    pub microphone: Arc<StdMutex<Option<VirtualMicrophone>>>,
    pub media_router: Arc<StdMutex<Option<MediaRouter>>>,
}

impl Default for VirtualDeviceState {
    fn default() -> Self {
        Self {
            webcam: Arc::new(StdMutex::new(None)),
            microphone: Arc::new(StdMutex::new(None)),
            media_router: Arc::new(StdMutex::new(None)),
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
pub async fn initialize_webcam(
    backend: String,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Initializing virtual webcam with backend: {}", backend);

    let backend = match backend.as_str() {
        "DirectShow" => WebcamBackend::DirectShow,
        "MediaFoundation" => WebcamBackend::MediaFoundation,
        _ => return Err("Invalid webcam backend. Use 'DirectShow' or 'MediaFoundation'".to_string()),
    };

    let webcam = VirtualWebcam::with_backend(backend);

    if let Err(e) = webcam.initialize().await {
        error!("Failed to initialize virtual webcam: {}", e);
        return Err(format!("Failed to initialize virtual webcam: {}", e));
    }

    let mut webcam_state = state.webcam.lock().map_err(|e| format!("Failed to lock webcam state: {}", e))?;
    *webcam_state = Some(webcam);

    info!("Virtual webcam initialized successfully");
    Ok(())
}

/// Initialize virtual microphone device
#[command]
pub async fn initialize_microphone(
    backend: String,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Initializing virtual microphone with backend: {}", backend);

    let backend = match backend.as_str() {
        "WASAPI" => MicrophoneBackend::WASAPI,
        "KernelStreaming" => MicrophoneBackend::KernelStreaming,
        _ => return Err("Invalid microphone backend. Use 'WASAPI' or 'KernelStreaming'".to_string()),
    };

    let microphone = VirtualMicrophone::with_backend(backend);

    if let Err(e) = microphone.initialize().await {
        error!("Failed to initialize virtual microphone: {}", e);
        return Err(format!("Failed to initialize virtual microphone: {}", e));
    }

    let mut microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    *microphone_state = Some(microphone);

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

    let webcam_state = state.webcam.lock().map_err(|e| format!("Failed to lock webcam state: {}", e))?;
    let webcam = webcam_state.as_ref().ok_or("Virtual webcam not initialized")?;

    if let Err(e) = webcam.start_streaming(&video_path).await {
        error!("Failed to start webcam streaming: {}", e);
        return Err(format!("Failed to start webcam streaming: {}", e));
    }

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

    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    if let Err(e) = microphone.start_streaming(&audio_path).await {
        error!("Failed to start microphone streaming: {}", e);
        return Err(format!("Failed to start microphone streaming: {}", e));
    }

    info!("Microphone streaming started successfully");
    Ok(())
}

/// Stop virtual webcam streaming
#[command]
pub async fn stop_webcam_streaming(
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Stopping webcam streaming");

    let webcam_state = state.webcam.lock().map_err(|e| format!("Failed to lock webcam state: {}", e))?;
    let webcam = webcam_state.as_ref().ok_or("Virtual webcam not initialized")?;

    if let Err(e) = webcam.stop_streaming().await {
        error!("Failed to stop webcam streaming: {}", e);
        return Err(format!("Failed to stop webcam streaming: {}", e));
    }

    info!("Webcam streaming stopped successfully");
    Ok(())
}

/// Stop virtual microphone streaming
#[command]
pub async fn stop_microphone_streaming(
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Stopping microphone streaming");

    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    if let Err(e) = microphone.stop_streaming().await {
        error!("Failed to stop microphone streaming: {}", e);
        return Err(format!("Failed to stop microphone streaming: {}", e));
    }

    info!("Microphone streaming stopped successfully");
    Ok(())
}

/// Get virtual device status
#[command]
pub async fn get_virtual_device_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<VirtualDeviceStatus, String> {
    let webcam_state = state.webcam.lock().map_err(|e| format!("Failed to lock webcam state: {}", e))?;
    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let router_state = state.media_router.lock().map_err(|e| format!("Failed to lock media router state: {}", e))?;

    let webcam_active = webcam_state.as_ref().map_or(false, |w| {
        futures::executor::block_on(w.is_active())
    });

    let microphone_active = microphone_state.as_ref().map_or(false, |m| {
        futures::executor::block_on(m.is_active())
    });

    let media_router_active = router_state.as_ref().map_or(false, |r| {
        futures::executor::block_on(r.get_status()).is_active
    });

    let webcam_backend = webcam_state.as_ref().map(|_| "DirectShow".to_string());
    let microphone_backend = microphone_state.as_ref().map(|_| "WASAPI".to_string());

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

    // Get webcam devices
    match VirtualWebcam::list_devices().await {
        Ok(webcam_devices) => {
            devices.push("Webcam Devices:".to_string());
            devices.extend(webcam_devices);
        },
        Err(e) => warn!("Failed to enumerate webcam devices: {}", e),
    }

    // Get microphone devices
    match VirtualMicrophone::list_devices().await {
        Ok(mic_devices) => {
            devices.push("Microphone Devices:".to_string());
            devices.extend(mic_devices);
        },
        Err(e) => warn!("Failed to enumerate microphone devices: {}", e),
    }

    // Add our virtual devices
    devices.extend([
        "VirtualWebcam (DirectShow)".to_string(),
        "VirtualWebcam (MediaFoundation)".to_string(),
        "VirtualMicrophone (WASAPI)".to_string(),
        "VirtualMicrophone (KernelStreaming)".to_string(),
    ]);

    Ok(devices)
}

/// Initialize media router
#[command]
pub async fn initialize_media_router(
    video_backend: String,
    audio_backend: String,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Initializing media router with video backend: {}, audio backend: {}", video_backend, audio_backend);

    let video_backend = match video_backend.as_str() {
        "DirectShow" => WebcamBackend::DirectShow,
        "MediaFoundation" => WebcamBackend::MediaFoundation,
        _ => return Err("Invalid video backend. Use 'DirectShow' or 'MediaFoundation'".to_string()),
    };

    let audio_backend = match audio_backend.as_str() {
        "WASAPI" => MicrophoneBackend::WASAPI,
        "KernelStreaming" => MicrophoneBackend::KernelStreaming,
        _ => return Err("Invalid audio backend. Use 'WASAPI' or 'KernelStreaming'".to_string()),
    };

    let config = MediaRoutingConfig {
        video_backend,
        audio_backend,
        ..Default::default()
    };

    let media_router = MediaRouter::with_config(config);

    if let Err(e) = media_router.initialize().await {
        error!("Failed to initialize media router: {}", e);
        return Err(format!("Failed to initialize media router: {}", e));
    }

    let mut router_state = state.media_router.lock().map_err(|e| format!("Failed to lock media router state: {}", e))?;
    *router_state = Some(media_router);

    info!("Media router initialized successfully");
    Ok(())
}

/// Start media routing
#[command]
pub async fn start_media_routing(
    video_path: Option<String>,
    audio_path: Option<String>,
    sync_audio_video: bool,
    loop_media: bool,
    video_volume: f32,
    audio_volume: f32,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Starting media routing");

    let router_state = state.media_router.lock().map_err(|e| format!("Failed to lock media router state: {}", e))?;
    let media_router = router_state.as_ref().ok_or("Media router not initialized")?;

    // Get current configuration
    let current_config = futures::executor::block_on(media_router.get_status()).config;

    let config = MediaRoutingConfig {
        video_path: video_path.unwrap_or_default(),
        audio_path: audio_path.unwrap_or_default(),
        sync_audio_video,
        loop_media,
        video_volume,
        audio_volume,
        ..current_config
    };

    if let Err(e) = media_router.start(config).await {
        error!("Failed to start media routing: {}", e);
        return Err(format!("Failed to start media routing: {}", e));
    }

    info!("Media routing started successfully");
    Ok(())
}

/// Stop media routing
#[command]
pub async fn stop_media_routing(
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Stopping media routing");

    let router_state = state.media_router.lock().map_err(|e| format!("Failed to lock media router state: {}", e))?;
    let media_router = router_state.as_ref().ok_or("Media router not initialized")?;

    if let Err(e) = media_router.stop().await {
        error!("Failed to stop media routing: {}", e);
        return Err(format!("Failed to stop media routing: {}", e));
    }

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
    info!("Switching media - video: {:?}, audio: {:?}", video_path, audio_path);

    let router_state = state.media_router.lock().map_err(|e| format!("Failed to lock media router state: {}", e))?;
    let media_router = router_state.as_ref().ok_or("Media router not initialized")?;

    if let Err(e) = media_router.switch_media(video_path, audio_path).await {
        error!("Failed to switch media: {}", e);
        return Err(format!("Failed to switch media: {}", e));
    }

    info!("Media switched successfully");
    Ok(())
}

/// Set microphone volume
#[command]
pub async fn set_microphone_volume(
    volume: f32,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Setting microphone volume to: {}", volume);

    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    if let Err(e) = microphone.set_volume(volume).await {
        error!("Failed to set microphone volume: {}", e);
        return Err(format!("Failed to set microphone volume: {}", e));
    }

    info!("Microphone volume set successfully");
    Ok(())
}

/// Get microphone volume
#[command]
pub async fn get_microphone_volume(
    state: State<'_, VirtualDeviceState>,
) -> Result<f32, String> {
    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    let volume = microphone.get_volume().await;
    Ok(volume)
}

/// Set microphone mute state
#[command]
pub async fn set_microphone_muted(
    muted: bool,
    state: State<'_, VirtualDeviceState>,
) -> Result<(), String> {
    info!("Setting microphone mute to: {}", muted);

    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    microphone.set_muted(muted).await;
    info!("Microphone mute set successfully");
    Ok(())
}

/// Get microphone mute state
#[command]
pub async fn get_microphone_muted(
    state: State<'_, VirtualDeviceState>,
) -> Result<bool, String> {
    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    let muted = microphone.is_muted().await;
    Ok(muted)
}

/// Get detailed media routing status
#[command]
pub async fn get_media_routing_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<crate::virtual::MediaRoutingStatus, String> {
    let router_state = state.media_router.lock().map_err(|e| format!("Failed to lock media router state: {}", e))?;
    let media_router = router_state.as_ref().ok_or("Media router not initialized")?;

    let status = media_router.get_status().await;
    Ok(status)
}

/// Get webcam video information
#[command]
pub async fn get_webcam_video_info(
    state: State<'_, VirtualDeviceState>,
) -> Result<Option<crate::virtual::VideoInfo>, String> {
    let webcam_state = state.webcam.lock().map_err(|e| format!("Failed to lock webcam state: {}", e))?;
    let webcam = webcam_state.as_ref().ok_or("Virtual webcam not initialized")?;

    let video_info = webcam.get_video_info().await.map_err(|e| format!("Failed to get video info: {}", e))?;
    Ok(video_info)
}

/// Get webcam buffer status
#[command]
pub async fn get_webcam_buffer_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<crate::virtual::BufferStatus, String> {
    let webcam_state = state.webcam.lock().map_err(|e| format!("Failed to lock webcam state: {}", e))?;
    let webcam = webcam_state.as_ref().ok_or("Virtual webcam not initialized")?;

    let buffer_status = webcam.get_buffer_status().await;
    Ok(buffer_status)
}

/// Get microphone buffer status
#[command]
pub async fn get_microphone_buffer_status(
    state: State<'_, VirtualDeviceState>,
) -> Result<(usize, usize, u64), String> {
    let microphone_state = state.microphone.lock().map_err(|e| format!("Failed to lock microphone state: {}", e))?;
    let microphone = microphone_state.as_ref().ok_or("Virtual microphone not initialized")?;

    let buffer_status = microphone.get_buffer_status().await;
    Ok(buffer_status)
}