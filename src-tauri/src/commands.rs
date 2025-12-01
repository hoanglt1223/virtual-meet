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

/// Shared application state
pub struct AppState {
    pub webcam: Arc<VirtualWebcam>,
}

/// Initialize the application state
pub fn init_state() -> AppState {
    AppState {
        webcam: Arc::new(VirtualWebcam::new()),
    }
}

/// Video file request
#[derive(Debug, Deserialize)]
pub struct VideoRequest {
    pub path: String,
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