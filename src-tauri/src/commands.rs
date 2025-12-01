//! Tauri Commands for VirtualMeet
//!
//! This module provides the API endpoints that the frontend can call
//! to control the virtual webcam and microphone functionality.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tracing::{info, error, warn};

use crate::virtual::webcam::{VirtualWebcam, VideoInfo, BufferStatus};
use crate::virtual::microphone::VirtualMicrophone;
use crate::audio::{AudioMetadata, AudioValidator};
use crate::audio_processor::{AudioProcessorStats, AudioVisualizationData};

/// Shared application state
pub struct AppState {
    pub webcam: Arc<VirtualWebcam>,
    pub microphone: Arc<VirtualMicrophone>,
}

/// Initialize the application state
pub fn init_state() -> AppState {
    AppState {
        webcam: Arc::new(VirtualWebcam::new()),
        microphone: Arc::new(VirtualMicrophone::new()),
    }
}

/// Video file request
#[derive(Debug, Deserialize)]
pub struct VideoRequest {
    pub path: String,
}

/// Audio file request
#[derive(Debug, Deserialize)]
pub struct AudioRequest {
    pub path: String,
}

/// Volume control request
#[derive(Debug, Deserialize)]
pub struct VolumeRequest {
    pub volume: f32,
}

/// Mute control request
#[derive(Debug, Deserialize)]
pub struct MuteRequest {
    pub muted: bool,
}

/// Video response with metadata
#[derive(Debug, Serialize)]
pub struct VideoResponse {
    pub success: bool,
    pub message: String,
    pub video_info: Option<VideoInfo>,
    pub buffer_status: Option<BufferStatus>,
}

/// Status response
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub is_active: bool,
    pub current_source: Option<String>,
    pub video_info: Option<VideoInfo>,
    pub buffer_status: Option<BufferStatus>,
}

/// Devices response
#[derive(Debug, Serialize)]
pub struct DevicesResponse {
    pub success: bool,
    pub devices: Vec<String>,
}

/// Audio metadata response
#[derive(Debug, Serialize)]
pub struct AudioMetadataResponse {
    pub success: bool,
    pub message: String,
    pub metadata: Option<AudioMetadata>,
}

/// Audio status response
#[derive(Debug, Serialize)]
pub struct AudioStatusResponse {
    pub is_active: bool,
    pub current_source: Option<String>,
    pub volume: f32,
    pub is_muted: bool,
    pub processing_stats: Option<AudioProcessorStats>,
    pub buffer_status: Option<(usize, usize, u64)>,
    pub visualization_data: Option<AudioVisualizationData>,
}

/// Initialize the virtual webcam
#[tauri::command]
pub async fn init_webcam(state: State<'_, AppState>) -> Result<VideoResponse, String> {
    info!("Initializing virtual webcam");

    match state.webcam.initialize().await {
        Ok(()) => {
            let video_info = state.webcam.get_video_info().await.unwrap_or(None);
            let buffer_status = state.webcam.get_buffer_status().await;

            Ok(VideoResponse {
                success: true,
                message: "Virtual webcam initialized successfully".to_string(),
                video_info,
                buffer_status: Some(buffer_status),
            })
        },
        Err(e) => {
            error!("Failed to initialize virtual webcam: {}", e);
            Ok(VideoResponse {
                success: false,
                message: format!("Failed to initialize virtual webcam: {}", e),
                video_info: None,
                buffer_status: None,
            })
        }
    }
}

/// Start video streaming from a file
#[tauri::command]
pub async fn start_streaming(
    request: VideoRequest,
    state: State<'_, AppState>
) -> Result<VideoResponse, String> {
    info!("Starting video streaming from: {}", request.path);

    match state.webcam.start_streaming(&request.path).await {
        Ok(()) => {
            let video_info = state.webcam.get_video_info().await.unwrap_or(None);
            let buffer_status = state.webcam.get_buffer_status().await;

            Ok(VideoResponse {
                success: true,
                message: format!("Started streaming from: {}", request.path),
                video_info,
                buffer_status: Some(buffer_status),
            })
        },
        Err(e) => {
            error!("Failed to start streaming: {}", e);
            Ok(VideoResponse {
                success: false,
                message: format!("Failed to start streaming: {}", e),
                video_info: None,
                buffer_status: None,
            })
        }
    }
}

/// Stop video streaming
#[tauri::command]
pub async fn stop_streaming(state: State<'_, AppState>) -> Result<VideoResponse, String> {
    info!("Stopping video streaming");

    match state.webcam.stop_streaming().await {
        Ok(()) => {
            let buffer_status = state.webcam.get_buffer_status().await;

            Ok(VideoResponse {
                success: true,
                message: "Video streaming stopped".to_string(),
                video_info: None,
                buffer_status: Some(buffer_status),
            })
        },
        Err(e) => {
            error!("Failed to stop streaming: {}", e);
            Ok(VideoResponse {
                success: false,
                message: format!("Failed to stop streaming: {}", e),
                video_info: None,
                buffer_status: None,
            })
        }
    }
}

/// Get webcam status
#[tauri::command]
pub async fn get_webcam_status(state: State<'_, AppState>) -> Result<StatusResponse, String> {
    let is_active = state.webcam.is_active().await;
    let current_source = state.webcam.current_source().await;
    let video_info = state.webcam.get_video_info().await.unwrap_or(None);
    let buffer_status = state.webcam.get_buffer_status().await;

    Ok(StatusResponse {
        is_active,
        current_source,
        video_info,
        buffer_status: Some(buffer_status),
    })
}

/// List available video devices
#[tauri::command]
pub async fn list_video_devices() -> Result<DevicesResponse, String> {
    info!("Listing video devices");

    match VirtualWebcam::list_devices().await {
        Ok(devices) => {
            Ok(DevicesResponse {
                success: true,
                devices,
            })
        },
        Err(e) => {
            error!("Failed to list video devices: {}", e);
            Ok(DevicesResponse {
                success: false,
                devices: vec![],
            })
        }
    }
}

/// Check if a video file is valid and can be decoded
#[tauri::command]
pub async fn validate_video_file(path: String) -> Result<VideoResponse, String> {
    info!("Validating video file: {}", path);

    // Create a temporary decoder to validate the file
    let mut decoder = crate::virtual::webcam::VideoDecoder::new();

    match decoder.open(&path) {
        Ok(()) => {
            let video_info = VideoInfo {
                width: decoder.width(),
                height: decoder.height(),
                frame_rate: decoder.frame_rate(),
                duration: decoder.duration(),
            };

            Ok(VideoResponse {
                success: true,
                message: format!("Video file is valid: {}x{} @ {:.2} FPS",
                               video_info.width, video_info.height, video_info.frame_rate),
                video_info: Some(video_info),
                buffer_status: None,
            })
        },
        Err(e) => {
            error!("Failed to validate video file: {}", e);
            Ok(VideoResponse {
                success: false,
                message: format!("Invalid video file: {}", e),
                video_info: None,
                buffer_status: None,
            })
        }
    }
}

// ============================================================================
// AUDIO COMMANDS
// ============================================================================

/// Initialize the virtual microphone
#[tauri::command]
pub async fn init_microphone(state: State<'_, AppState>) -> Result<VideoResponse, String> {
    info!("Initializing virtual microphone");

    match state.microphone.initialize().await {
        Ok(()) => {
            Ok(VideoResponse {
                success: true,
                message: "Virtual microphone initialized successfully".to_string(),
                video_info: None,
                buffer_status: None,
            })
        },
        Err(e) => {
            error!("Failed to initialize virtual microphone: {}", e);
            Ok(VideoResponse {
                success: false,
                message: format!("Failed to initialize virtual microphone: {}", e),
                video_info: None,
                buffer_status: None,
            })
        }
    }
}

/// Start audio streaming from a file
#[tauri::command]
pub async fn start_audio_streaming(
    request: AudioRequest,
    state: State<'_, AppState>
) -> Result<VideoResponse, String> {
    info!("Starting audio streaming from: {}", request.path);

    // Note: We need to clone the Arc to get a mutable reference
    // This is a limitation of the current architecture and should be refactored
    // In a production system, you'd use interior mutability or a different pattern

    Ok(VideoResponse {
        success: false,
        message: "Audio streaming requires mutable state access - not implemented in this command structure".to_string(),
        video_info: None,
        buffer_status: None,
    })
}

/// Stop audio streaming
#[tauri::command]
pub async fn stop_audio_streaming(state: State<'_, AppState>) -> Result<VideoResponse, String> {
    info!("Stopping audio streaming");

    Ok(VideoResponse {
        success: false,
        message: "Audio streaming requires mutable state access - not implemented in this command structure".to_string(),
        video_info: None,
        buffer_status: None,
    })
}

/// Get microphone status
#[tauri::command]
pub async fn get_microphone_status(state: State<'_, AppState>) -> Result<AudioStatusResponse, String> {
    let is_active = state.microphone.is_active().await;
    let current_source = state.microphone.current_source().await;
    let volume = state.microphone.get_volume().await;
    let is_muted = state.microphone.is_muted().await;
    let processing_stats = Some(state.microphone.get_processing_stats().await);
    let buffer_status = Some(state.microphone.get_buffer_status().await);
    let visualization_data = Some(state.microphone.get_visualization_data().await);

    Ok(AudioStatusResponse {
        is_active,
        current_source,
        volume,
        is_muted,
        processing_stats,
        buffer_status,
        visualization_data,
    })
}

/// Set microphone volume
#[tauri::command]
pub async fn set_microphone_volume(
    request: VolumeRequest,
    state: State<'_, AppState>
) -> Result<VideoResponse, String> {
    info!("Setting microphone volume to: {}", request.volume);

    match state.microphone.set_volume(request.volume).await {
        Ok(()) => {
            Ok(VideoResponse {
                success: true,
                message: format!("Volume set to: {:.2}", request.volume),
                video_info: None,
                buffer_status: None,
            })
        },
        Err(e) => {
            error!("Failed to set volume: {}", e);
            Ok(VideoResponse {
                success: false,
                message: format!("Failed to set volume: {}", e),
                video_info: None,
                buffer_status: None,
            })
        }
    }
}

/// Set microphone mute state
#[tauri::command]
pub async fn set_microphone_muted(
    request: MuteRequest,
    state: State<'_, AppState>
) -> Result<VideoResponse, String> {
    info!("Setting microphone mute state to: {}", request.muted);

    state.microphone.set_muted(request.muted).await;

    Ok(VideoResponse {
        success: true,
        message: format!("Mute state set to: {}", request.muted),
        video_info: None,
        buffer_status: None,
    })
}

/// Toggle microphone mute state
#[tauri::command]
pub async fn toggle_microphone_mute(state: State<'_, AppState>) -> Result<VideoResponse, String> {
    info!("Toggling microphone mute state");

    let new_mute_state = state.microphone.toggle_mute().await;

    Ok(VideoResponse {
        success: true,
        message: format!("Mute toggled to: {}", new_mute_state),
        video_info: None,
        buffer_status: None,
    })
}

/// List available audio devices
#[tauri::command]
pub async fn list_audio_devices() -> Result<DevicesResponse, String> {
    info!("Listing audio devices");

    match VirtualMicrophone::list_devices().await {
        Ok(devices) => {
            Ok(DevicesResponse {
                success: true,
                devices,
            })
        },
        Err(e) => {
            error!("Failed to list audio devices: {}", e);
            Ok(DevicesResponse {
                success: false,
                devices: vec![],
            })
        }
    }
}

/// Check if an audio file is valid and can be decoded
#[tauri::command]
pub async fn validate_audio_file(path: String) -> Result<AudioMetadataResponse, String> {
    info!("Validating audio file: {}", path);

    // First validate the file format
    match AudioValidator::validate_audio_file(&path) {
        Ok(()) => {
            // Create a temporary decoder to validate the file
            let mut decoder = crate::audio_decoder::AudioDecoder::new();

            match decoder.open(&path) {
                Ok(()) => {
                    let metadata = decoder.get_metadata().clone();

                    Ok(AudioMetadataResponse {
                        success: true,
                        message: format!("Audio file is valid: {} channels, {} Hz, {}",
                                       metadata.channels, metadata.sample_rate, metadata.codec),
                        metadata: Some(metadata),
                    })
                },
                Err(e) => {
                    error!("Failed to decode audio file: {}", e);
                    Ok(AudioMetadataResponse {
                        success: false,
                        message: format!("Invalid audio file: {}", e),
                        metadata: None,
                    })
                }
            }
        },
        Err(e) => {
            error!("Failed to validate audio file: {}", e);
            Ok(AudioMetadataResponse {
                success: false,
                message: format!("Invalid audio file: {}", e),
                metadata: None,
            })
        }
    }
}

/// Get supported audio formats
#[tauri::command]
pub async fn get_supported_audio_formats() -> Result<DevicesResponse, String> {
    let formats = AudioValidator::supported_formats()
        .iter()
        .map(|s| s.to_string())
        .collect();

    Ok(DevicesResponse {
        success: true,
        devices: formats,
    })
}