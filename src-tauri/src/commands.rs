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
use crate::virtual::VirtualDeviceState;
use crate::audio::{AudioMetadata, AudioValidator};
use crate::audio_processor::{AudioProcessorStats, AudioVisualizationData};
use crate::devices::{
    DeviceEnumerator, DeviceFilter, DeviceFilterer, FullDeviceInfo,
    DeviceType, DeviceCategory, DeviceOrigin,
};

// Include virtual device commands
mod virtual_devices;
pub use virtual_devices::*;

/// Shared application state
pub struct AppState {
    pub webcam: Arc<VirtualWebcam>,
    pub microphone: Arc<VirtualMicrophone>,
    pub device_enumerator: Arc<DeviceEnumerator>,
}

/// Initialize the application state
pub fn init_state() -> AppState {
    AppState {
        webcam: Arc::new(VirtualWebcam::new()),
        microphone: Arc::new(VirtualMicrophone::new()),
        device_enumerator: Arc::new(DeviceEnumerator::new()),
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

/// Device enumeration filter request
#[derive(Debug, Deserialize)]
pub struct DeviceFilterRequest {
    pub device_type: Option<String>,
    pub category: Option<String>,
    pub origin: Option<String>,
    pub available_only: Option<bool>,
    pub virtual_only: Option<bool>,
    pub physical_only: Option<bool>,
    pub driver_contains: Option<String>,
    pub name_contains: Option<String>,
}

/// Device enumeration response
#[derive(Debug, Serialize)]
pub struct DeviceEnumerationResponse {
    pub success: bool,
    pub message: String,
    pub devices: Vec<FullDeviceInfo>,
    pub total_count: usize,
    pub virtual_count: usize,
    pub physical_count: usize,
    pub audio_count: usize,
    pub video_count: usize,
    pub timestamp: String,
}

/// Device capability response
#[derive(Debug, Serialize)]
pub struct DeviceCapabilityResponse {
    pub success: bool,
    pub message: String,
    pub device_id: String,
    pub capabilities: Option<crate::devices::DeviceCapabilities>,
}

/// Device virtual status response
#[derive(Debug, Serialize)]
pub struct DeviceVirtualStatusResponse {
    pub success: bool,
    pub message: String,
    pub device_id: String,
    pub is_virtual: Option<bool>,
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

// ============================================================================
// DEVICE ENUMERATION COMMANDS
// ============================================================================

/// Enumerate all available devices (audio and video)
#[tauri::command]
pub async fn enumerate_all_devices(
    filter: Option<DeviceFilterRequest>,
    state: State<'_, AppState>
) -> Result<DeviceEnumerationResponse, String> {
    info!("Enumerating all devices");

    match state.device_enumerator.enumerate_all_devices().await {
        Ok(mut enumeration_result) => {
            // Apply filter if provided
            let devices = if let Some(filter_request) = filter {
                let filter = convert_filter_request(filter_request);
                DeviceFilterer::filter_devices(&enumeration_result.devices, &filter)
                    .into_iter()
                    .cloned()
                    .collect()
            } else {
                enumeration_result.devices.clone()
            };

            let virtual_count = devices.iter().filter(|d| d.info.is_virtual()).count();
            let physical_count = devices.iter().filter(|d| d.info.is_physical()).count();
            let audio_count = devices.iter().filter(|d| d.info.device_type == DeviceType::Audio).count();
            let video_count = devices.iter().filter(|d| d.info.device_type == DeviceType::Video).count();

            Ok(DeviceEnumerationResponse {
                success: true,
                message: format!("Successfully enumerated {} devices", devices.len()),
                devices,
                total_count: devices.len(),
                virtual_count,
                physical_count,
                audio_count,
                video_count,
                timestamp: enumeration_result.timestamp.to_rfc3339(),
            })
        },
        Err(e) => {
            error!("Failed to enumerate devices: {}", e);
            Ok(DeviceEnumerationResponse {
                success: false,
                message: format!("Failed to enumerate devices: {}", e),
                devices: vec![],
                total_count: 0,
                virtual_count: 0,
                physical_count: 0,
                audio_count: 0,
                video_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Enumerate audio devices only
#[tauri::command]
pub async fn enumerate_audio_devices(
    filter: Option<DeviceFilterRequest>,
    state: State<'_, AppState>
) -> Result<DeviceEnumerationResponse, String> {
    info!("Enumerating audio devices");

    match state.device_enumerator.enumerate_audio_devices().await {
        Ok(mut enumeration_result) => {
            // Apply filter if provided
            let devices = if let Some(filter_request) = filter {
                let filter = convert_filter_request(filter_request);
                DeviceFilterer::filter_devices(&enumeration_result.devices, &filter)
                    .into_iter()
                    .cloned()
                    .collect()
            } else {
                enumeration_result.devices.clone()
            };

            let virtual_count = devices.iter().filter(|d| d.info.is_virtual()).count();
            let physical_count = devices.iter().filter(|d| d.info.is_physical()).count();

            Ok(DeviceEnumerationResponse {
                success: true,
                message: format!("Successfully enumerated {} audio devices", devices.len()),
                devices,
                total_count: devices.len(),
                virtual_count,
                physical_count,
                audio_count: devices.len(),
                video_count: 0,
                timestamp: enumeration_result.timestamp.to_rfc3339(),
            })
        },
        Err(e) => {
            error!("Failed to enumerate audio devices: {}", e);
            Ok(DeviceEnumerationResponse {
                success: false,
                message: format!("Failed to enumerate audio devices: {}", e),
                devices: vec![],
                total_count: 0,
                virtual_count: 0,
                physical_count: 0,
                audio_count: 0,
                video_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Enumerate video devices only
#[tauri::command]
pub async fn enumerate_video_devices(
    filter: Option<DeviceFilterRequest>,
    state: State<'_, AppState>
) -> Result<DeviceEnumerationResponse, String> {
    info!("Enumerating video devices");

    match state.device_enumerator.enumerate_video_devices().await {
        Ok(mut enumeration_result) => {
            // Apply filter if provided
            let devices = if let Some(filter_request) = filter {
                let filter = convert_filter_request(filter_request);
                DeviceFilterer::filter_devices(&enumeration_result.devices, &filter)
                    .into_iter()
                    .cloned()
                    .collect()
            } else {
                enumeration_result.devices.clone()
            };

            let virtual_count = devices.iter().filter(|d| d.info.is_virtual()).count();
            let physical_count = devices.iter().filter(|d| d.info.is_physical()).count();

            Ok(DeviceEnumerationResponse {
                success: true,
                message: format!("Successfully enumerated {} video devices", devices.len()),
                devices,
                total_count: devices.len(),
                virtual_count,
                physical_count,
                audio_count: 0,
                video_count: devices.len(),
                timestamp: enumeration_result.timestamp.to_rfc3339(),
            })
        },
        Err(e) => {
            error!("Failed to enumerate video devices: {}", e);
            Ok(DeviceEnumerationResponse {
                success: false,
                message: format!("Failed to enumerate video devices: {}", e),
                devices: vec![],
                total_count: 0,
                virtual_count: 0,
                physical_count: 0,
                audio_count: 0,
                video_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Get device capabilities
#[tauri::command]
pub async fn get_device_capabilities(
    device_id: String,
    state: State<'_, AppState>
) -> Result<DeviceCapabilityResponse, String> {
    info!("Getting capabilities for device: {}", device_id);

    match state.device_enumerator.get_device_capabilities(&device_id).await {
        Ok(capabilities) => {
            Ok(DeviceCapabilityResponse {
                success: true,
                message: format!("Successfully retrieved capabilities for device: {}", device_id),
                device_id,
                capabilities: Some(capabilities),
            })
        },
        Err(e) => {
            error!("Failed to get device capabilities: {}", e);
            Ok(DeviceCapabilityResponse {
                success: false,
                message: format!("Failed to get device capabilities: {}", e),
                device_id,
                capabilities: None,
            })
        }
    }
}

/// Check if a device is virtual
#[tauri::command]
pub async fn is_device_virtual(
    device_id: String,
    state: State<'_, AppState>
) -> Result<DeviceVirtualStatusResponse, String> {
    info!("Checking virtual status for device: {}", device_id);

    match state.device_enumerator.is_virtual_device(&device_id).await {
        Ok(is_virtual) => {
            Ok(DeviceVirtualStatusResponse {
                success: true,
                message: format!("Device {} virtual status: {}", device_id, is_virtual),
                device_id,
                is_virtual: Some(is_virtual),
            })
        },
        Err(e) => {
            error!("Failed to check device virtual status: {}", e);
            Ok(DeviceVirtualStatusResponse {
                success: false,
                message: format!("Failed to check device virtual status: {}", e),
                device_id,
                is_virtual: None,
            })
        }
    }
}

/// Get virtual devices only
#[tauri::command]
pub async fn get_virtual_devices(
    device_type: Option<String>,
    state: State<'_, AppState>
) -> Result<DeviceEnumerationResponse, String> {
    info!("Getting virtual devices");

    match state.device_enumerator.enumerate_all_devices().await {
        Ok(enumeration_result) => {
            let mut virtual_devices = enumeration_result.get_virtual_devices()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();

            // Filter by device type if specified
            if let Some(device_type_str) = device_type {
                if let Ok(device_type) = parse_device_type(&device_type_str) {
                    virtual_devices.retain(|d| d.info.device_type == device_type);
                }
            }

            let audio_count = virtual_devices.iter().filter(|d| d.info.device_type == DeviceType::Audio).count();
            let video_count = virtual_devices.iter().filter(|d| d.info.device_type == DeviceType::Video).count();

            Ok(DeviceEnumerationResponse {
                success: true,
                message: format!("Found {} virtual devices", virtual_devices.len()),
                devices: virtual_devices,
                total_count: virtual_devices.len(),
                virtual_count: virtual_devices.len(),
                physical_count: 0,
                audio_count,
                video_count,
                timestamp: enumeration_result.timestamp.to_rfc3339(),
            })
        },
        Err(e) => {
            error!("Failed to get virtual devices: {}", e);
            Ok(DeviceEnumerationResponse {
                success: false,
                message: format!("Failed to get virtual devices: {}", e),
                devices: vec![],
                total_count: 0,
                virtual_count: 0,
                physical_count: 0,
                audio_count: 0,
                video_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Get physical devices only
#[tauri::command]
pub async fn get_physical_devices(
    device_type: Option<String>,
    state: State<'_, AppState>
) -> Result<DeviceEnumerationResponse, String> {
    info!("Getting physical devices");

    match state.device_enumerator.enumerate_all_devices().await {
        Ok(enumeration_result) => {
            let mut physical_devices = enumeration_result.get_physical_devices()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();

            // Filter by device type if specified
            if let Some(device_type_str) = device_type {
                if let Ok(device_type) = parse_device_type(&device_type_str) {
                    physical_devices.retain(|d| d.info.device_type == device_type);
                }
            }

            let audio_count = physical_devices.iter().filter(|d| d.info.device_type == DeviceType::Audio).count();
            let video_count = physical_devices.iter().filter(|d| d.info.device_type == DeviceType::Video).count();

            Ok(DeviceEnumerationResponse {
                success: true,
                message: format!("Found {} physical devices", physical_devices.len()),
                devices: physical_devices,
                total_count: physical_devices.len(),
                virtual_count: 0,
                physical_count: physical_devices.len(),
                audio_count,
                video_count,
                timestamp: enumeration_result.timestamp.to_rfc3339(),
            })
        },
        Err(e) => {
            error!("Failed to get physical devices: {}", e);
            Ok(DeviceEnumerationResponse {
                success: false,
                message: format!("Failed to get physical devices: {}", e),
                devices: vec![],
                total_count: 0,
                virtual_count: 0,
                physical_count: 0,
                audio_count: 0,
                video_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Refresh device list
#[tauri::command]
pub async fn refresh_device_list(
    device_type: Option<String>,
    state: State<'_, AppState>
) -> Result<DeviceEnumerationResponse, String> {
    info!("Refreshing device list");

    let result = match device_type.as_deref() {
        Some("audio") => state.device_enumerator.enumerate_audio_devices().await,
        Some("video") => state.device_enumerator.enumerate_video_devices().await,
        _ => state.device_enumerator.enumerate_all_devices().await,
    };

    match result {
        Ok(enumeration_result) => {
            let devices = enumeration_result.devices;
            let virtual_count = devices.iter().filter(|d| d.info.is_virtual()).count();
            let physical_count = devices.iter().filter(|d| d.info.is_physical()).count();
            let audio_count = devices.iter().filter(|d| d.info.device_type == DeviceType::Audio).count();
            let video_count = devices.iter().filter(|d| d.info.device_type == DeviceType::Video).count();

            Ok(DeviceEnumerationResponse {
                success: true,
                message: "Device list refreshed successfully".to_string(),
                devices,
                total_count: devices.len(),
                virtual_count,
                physical_count,
                audio_count,
                video_count,
                timestamp: enumeration_result.timestamp.to_rfc3339(),
            })
        },
        Err(e) => {
            error!("Failed to refresh device list: {}", e);
            Ok(DeviceEnumerationResponse {
                success: false,
                message: format!("Failed to refresh device list: {}", e),
                devices: vec![],
                total_count: 0,
                virtual_count: 0,
                physical_count: 0,
                audio_count: 0,
                video_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Helper function to convert filter request to internal filter type
fn convert_filter_request(request: DeviceFilterRequest) -> DeviceFilter {
    DeviceFilter {
        device_type: request.device_type.and_then(|s| parse_device_type(&s).ok()),
        category: request.category.and_then(|s| parse_device_category(&s).ok()),
        origin: request.origin.and_then(|s| parse_device_origin(&s).ok()),
        available_only: request.available_only.unwrap_or(false),
        virtual_only: request.virtual_only.unwrap_or(false),
        physical_only: request.physical_only.unwrap_or(false),
        driver_contains: request.driver_contains,
        name_contains: request.name_contains,
    }
}

/// Parse device type from string
fn parse_device_type(s: &str) -> Result<DeviceType, String> {
    match s.to_lowercase().as_str() {
        "audio" => Ok(DeviceType::Audio),
        "video" => Ok(DeviceType::Video),
        _ => Err(format!("Invalid device type: {}", s)),
    }
}

/// Parse device category from string
fn parse_device_category(s: &str) -> Result<DeviceCategory, String> {
    match s.to_lowercase().as_str() {
        "input" => Ok(DeviceCategory::Input),
        "output" => Ok(DeviceCategory::Output),
        "both" => Ok(DeviceCategory::Both),
        _ => Err(format!("Invalid device category: {}", s)),
    }
}

/// Parse device origin from string
fn parse_device_origin(s: &str) -> Result<DeviceOrigin, String> {
    match s.to_lowercase().as_str() {
        "physical" => Ok(DeviceOrigin::Physical),
        "virtual" => Ok(DeviceOrigin::Virtual),
        "unknown" => Ok(DeviceOrigin::Unknown),
        _ => Err(format!("Invalid device origin: {}", s)),
    }
}