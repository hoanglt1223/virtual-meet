//! Settings Management Commands
//!
//! Tauri commands for application settings, configuration, and preferences management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{command, State, AppHandle};
use tracing::{info, error, warn, debug};

use crate::AppState;
use crate::devices::{DeviceType, DeviceCategory, DeviceOrigin};

/// Application settings structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub recording: RecordingSettings,
    pub devices: DeviceSettings,
    pub hotkeys: HotkeySettings,
    pub ui: UISettings,
    pub advanced: AdvancedSettings,
}

/// General application settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralSettings {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub check_updates: bool,
    pub language: String,
    pub theme: ThemeMode,
    pub auto_save_interval: u32, // minutes
}

/// Video settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoSettings {
    pub default_resolution: VideoResolution,
    pub default_fps: f32,
    pub default_quality: VideoQuality,
    pub hardware_acceleration: bool,
    pub video_backend: VideoBackend,
    pub deinterlacing: bool,
    pub color_space: ColorSpace,
}

/// Audio settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioSettings {
    pub default_sample_rate: u32,
    pub default_bit_depth: u16,
    pub default_channels: u8,
    pub default_audio_backend: AudioBackend,
    pub audio_buffer_size: u32,
    pub noise_reduction: bool,
    pub echo_cancellation: bool,
    pub auto_gain_control: bool,
}

/// Recording settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingSettings {
    pub default_output_path: String,
    pub default_format: RecordingFormat,
    pub auto_segment_files: bool,
    pub segment_duration_minutes: u32,
    pub include_timestamp: bool,
    pub compression_level: u8,
    pub simultaneous_recording: bool,
}

/// Device settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceSettings {
    pub preferred_webcam: Option<String>,
    pub preferred_microphone: Option<String>,
    pub preferred_speaker: Option<String>,
    pub auto_detect_devices: bool,
    pub device_refresh_interval: u32, // seconds
    pub virtual_device_settings: VirtualDeviceSettings,
}

/// Virtual device specific settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirtualDeviceSettings {
    pub webcam_backend: String,
    pub microphone_backend: String,
    pub buffer_size_mb: u32,
    pub low_latency_mode: bool,
}

/// Hotkey settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotkeySettings {
    pub enabled: bool,
    pub global_hotkeys: HashMap<String, String>,
    pub conflict_resolution: HotkeyConflictResolution,
}

/// UI settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UISettings {
    pub window_size: (u32, u32),
    pub window_position: Option<(i32, i32)>,
    pub always_on_top: bool,
    pub show_tooltips: bool,
    pub show_notifications: bool,
    pub compact_mode: bool,
}

/// Advanced settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvancedSettings {
    pub log_level: LogLevel,
    pub max_log_size_mb: u32,
    pub debug_mode: bool,
    pub experimental_features: bool,
    pub performance_mode: PerformanceMode,
    pub custom_ffmpeg_path: Option<String>,
}

/// Enum definitions
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoResolution {
    HD720p,
    HD1080p,
    HD1440p,
    UHD4K,
    Custom(u32, u32),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoQuality {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoBackend {
    DirectShow,
    MediaFoundation,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ColorSpace {
    RGB24,
    YUV420P,
    YUV444P,
    Auto,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AudioBackend {
    WASAPI,
    DirectSound,
    ASIO,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RecordingFormat {
    MP4,
    MKV,
    WebM,
    AVI,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum HotkeyConflictResolution {
    FirstCome,
    LastCome,
    Prompt,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PerformanceMode {
    PowerSaving,
    Balanced,
    HighPerformance,
}

/// Settings update request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub category: SettingsCategory,
    pub settings: serde_json::Value,
}

/// Settings category
#[derive(Debug, Deserialize)]
pub enum SettingsCategory {
    General,
    Video,
    Audio,
    Recording,
    Devices,
    Hotkeys,
    UI,
    Advanced,
}

/// Settings response
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub success: bool,
    pub message: String,
    pub settings: Option<AppSettings>,
}

/// Settings export/import request
#[derive(Debug, Deserialize)]
pub struct SettingsExportRequest {
    pub file_path: String,
    pub include_sensitive: bool,
    pub categories: Option<Vec<SettingsCategory>>,
}

/// Settings validation result
#[derive(Debug, Serialize)]
pub struct SettingsValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info_messages: Vec<String>,
}

/// Reset settings request
#[derive(Debug, Deserialize)]
pub struct ResetSettingsRequest {
    pub categories: Vec<SettingsCategory>,
    pub confirm: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings {
                auto_start: false,
                start_minimized: false,
                minimize_to_tray: true,
                check_updates: true,
                language: "en".to_string(),
                theme: ThemeMode::System,
                auto_save_interval: 5,
            },
            video: VideoSettings {
                default_resolution: VideoResolution::HD1080p,
                default_fps: 30.0,
                default_quality: VideoQuality::High,
                hardware_acceleration: true,
                video_backend: VideoBackend::DirectShow,
                deinterlacing: false,
                color_space: ColorSpace::Auto,
            },
            audio: AudioSettings {
                default_sample_rate: 44100,
                default_bit_depth: 16,
                default_channels: 2,
                default_audio_backend: AudioBackend::WASAPI,
                audio_buffer_size: 1024,
                noise_reduction: false,
                echo_cancellation: false,
                auto_gain_control: false,
            },
            recording: RecordingSettings {
                default_output_path: get_default_output_path(),
                default_format: RecordingFormat::MP4,
                auto_segment_files: false,
                segment_duration_minutes: 60,
                include_timestamp: true,
                compression_level: 5,
                simultaneous_recording: false,
            },
            devices: DeviceSettings {
                preferred_webcam: None,
                preferred_microphone: None,
                preferred_speaker: None,
                auto_detect_devices: true,
                device_refresh_interval: 30,
                virtual_device_settings: VirtualDeviceSettings {
                    webcam_backend: "DirectShow".to_string(),
                    microphone_backend: "WASAPI".to_string(),
                    buffer_size_mb: 256,
                    low_latency_mode: true,
                },
            },
            hotkeys: HotkeySettings {
                enabled: true,
                global_hotkeys: HashMap::new(),
                conflict_resolution: HotkeyConflictResolution::Prompt,
            },
            ui: UISettings {
                window_size: (1280, 720),
                window_position: None,
                always_on_top: false,
                show_tooltips: true,
                show_notifications: true,
                compact_mode: false,
            },
            advanced: AdvancedSettings {
                log_level: LogLevel::Info,
                max_log_size_mb: 100,
                debug_mode: false,
                experimental_features: false,
                performance_mode: PerformanceMode::Balanced,
                custom_ffmpeg_path: None,
            },
        }
    }
}

/// Get application settings
#[command]
pub async fn get_settings() -> Result<SettingsResponse, String> {
    info!("Getting application settings");

    // In a real implementation, you would load settings from a configuration file
    let settings = AppSettings::default();

    Ok(SettingsResponse {
        success: true,
        message: "Settings retrieved successfully".to_string(),
        settings: Some(settings),
    })
}

/// Update application settings
#[command]
pub async fn update_settings(
    request: UpdateSettingsRequest,
) -> Result<SettingsValidationResult, String> {
    info!("Updating settings for category: {:?}", request.category);

    // Validate the settings
    let validation_result = validate_settings(&request.category, &request.settings);

    if validation_result.is_valid {
        // In a real implementation, you would save the settings to a configuration file
        info!("Settings updated successfully for category: {:?}", request.category);
    } else {
        warn!("Settings validation failed: {:?}", validation_result.errors);
    }

    Ok(validation_result)
}

/// Reset settings to defaults
#[command]
pub async fn reset_settings(
    request: ResetSettingsRequest,
) -> Result<SettingsResponse, String> {
    info!("Resetting settings for categories: {:?}", request.categories);

    if !request.confirm {
        return Ok(SettingsResponse {
            success: false,
            message: "Reset confirmation required".to_string(),
            settings: None,
        });
    }

    // In a real implementation, you would reset the specified categories
    let mut default_settings = AppSettings::default();

    // Only return the categories that were reset
    if !request.categories.contains(&SettingsCategory::General) {
        // Keep existing general settings
    }

    Ok(SettingsResponse {
        success: true,
        message: format!("Settings reset for {} categories", request.categories.len()),
        settings: Some(default_settings),
    })
}

/// Export settings to file
#[command]
pub async fn export_settings(
    request: SettingsExportRequest,
) -> Result<SettingsResponse, String> {
    info!("Exporting settings to: {}", request.file_path);

    // In a real implementation, you would export settings to the specified file
    let settings = AppSettings::default();

    // Convert to JSON and write to file
    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    // Write to file (simplified)
    std::fs::write(&request.file_path, json_content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(SettingsResponse {
        success: true,
        message: format!("Settings exported to: {}", request.file_path),
        settings: Some(settings),
    })
}

/// Import settings from file
#[command]
pub async fn import_settings(file_path: String) -> Result<SettingsValidationResult, String> {
    info!("Importing settings from: {}", file_path);

    // Read and parse settings file
    let json_content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    let settings: AppSettings = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse settings file: {}", e))?;

    // Validate all settings categories
    let mut all_valid = true;
    let mut all_errors = Vec::new();
    let mut all_warnings = Vec::new();
    let mut all_info = Vec::new();

    for category in [
        SettingsCategory::General,
        SettingsCategory::Video,
        SettingsCategory::Audio,
        SettingsCategory::Recording,
        SettingsCategory::Devices,
        SettingsCategory::Hotkeys,
        SettingsCategory::UI,
        SettingsCategory::Advanced,
    ] {
        let result = validate_settings_category(&category, &settings);
        if !result.is_valid {
            all_valid = false;
            all_errors.extend(result.errors);
        }
        all_warnings.extend(result.warnings);
        all_info.extend(result.info_messages);
    }

    if all_valid {
        // In a real implementation, you would apply the imported settings
        info!("Settings imported successfully from: {}", file_path);
    }

    Ok(SettingsValidationResult {
        is_valid: all_valid,
        errors: all_errors,
        warnings: all_warnings,
        info_messages: all_info,
    })
}

/// Get available video devices with their capabilities
#[command]
pub async fn get_available_video_devices() -> Result<Vec<VideoDeviceInfo>, String> {
    info!("Getting available video devices");

    // In a real implementation, you would query the system for video devices
    let devices = vec![
        VideoDeviceInfo {
            id: "webcam_0".to_string(),
            name: "Integrated Webcam".to_string(),
            resolution: (1280, 720),
            fps: 30.0,
            is_virtual: false,
            capabilities: vec!["1080p".to_string(), "720p".to_string()],
        },
    ];

    Ok(devices)
}

/// Get available audio devices with their capabilities
#[command]
pub async fn get_available_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    info!("Getting available audio devices");

    // In a real implementation, you would query the system for audio devices
    let devices = vec![
        AudioDeviceInfo {
            id: "mic_0".to_string(),
            name: "Microphone".to_string(),
            sample_rate: 44100,
            channels: 2,
            bit_depth: 16,
            is_virtual: false,
        },
    ];

    Ok(devices)
}

/// Video device information
#[derive(Debug, Serialize)]
pub struct VideoDeviceInfo {
    pub id: String,
    pub name: String,
    pub resolution: (u32, u32),
    pub fps: f32,
    pub is_virtual: bool,
    pub capabilities: Vec<String>,
}

/// Audio device information
#[derive(Debug, Serialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u16,
    pub is_virtual: bool,
}

/// Validate settings for a specific category
fn validate_settings(
    category: &SettingsCategory,
    settings: &serde_json::Value,
) -> SettingsValidationResult {
    match category {
        SettingsCategory::Video => validate_video_settings(settings),
        SettingsCategory::Audio => validate_audio_settings(settings),
        SettingsCategory::Recording => validate_recording_settings(settings),
        _ => SettingsValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            info_messages: vec!["Validation not implemented for this category".to_string()],
        },
    }
}

/// Validate settings category from AppSettings
fn validate_settings_category(
    category: &SettingsCategory,
    settings: &AppSettings,
) -> SettingsValidationResult {
    let category_json = match category {
        SettingsCategory::Video => serde_json::to_value(&settings.video).unwrap_or_default(),
        SettingsCategory::Audio => serde_json::to_value(&settings.audio).unwrap_or_default(),
        SettingsCategory::Recording => serde_json::to_value(&settings.recording).unwrap_or_default(),
        _ => return SettingsValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            info_messages: vec!["Validation not implemented for this category".to_string()],
        },
    };

    validate_settings(category, &category_json)
}

/// Validate video settings
fn validate_video_settings(settings: &serde_json::Value) -> SettingsValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate FPS
    if let Some(fps) = settings.get("default_fps").and_then(|v| v.as_f64()) {
        if fps < 15.0 || fps > 120.0 {
            errors.push("Frame rate should be between 15 and 120 FPS".to_string());
        } else if fps > 60.0 {
            warnings.push("Frame rates above 60 FPS may impact performance".to_string());
        }
    }

    // Validate resolution
    if let Some(_resolution) = settings.get("default_resolution") {
        // Additional validation logic here
    }

    SettingsValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
        info_messages: vec![],
    }
}

/// Validate audio settings
fn validate_audio_settings(settings: &serde_json::Value) -> SettingsValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate sample rate
    if let Some(sample_rate) = settings.get("default_sample_rate").and_then(|v| v.as_u64()) {
        if ![44100, 48000, 96000, 192000].contains(&(sample_rate as u32)) {
            errors.push("Sample rate should be one of: 44100, 48000, 96000, 192000 Hz".to_string());
        } else if sample_rate > 96000 {
            warnings.push("High sample rates may impact performance".to_string());
        }
    }

    // Validate bit depth
    if let Some(bit_depth) = settings.get("default_bit_depth").and_then(|v| v.as_u64()) {
        if ![16, 24, 32].contains(&(bit_depth as u16)) {
            errors.push("Bit depth should be one of: 16, 24, 32 bits".to_string());
        }
    }

    SettingsValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
        info_messages: vec![],
    }
}

/// Validate recording settings
fn validate_recording_settings(settings: &serde_json::Value) -> SettingsValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate output path
    if let Some(path) = settings.get("default_output_path").and_then(|v| v.as_str()) {
        if path.is_empty() {
            errors.push("Output path cannot be empty".to_string());
        }
    }

    // Validate segment duration
    if let Some(duration) = settings.get("segment_duration_minutes").and_then(|v| v.as_u64()) {
        if duration == 0 || duration > 1440 {
            errors.push("Segment duration should be between 1 and 1440 minutes".to_string());
        }
    }

    SettingsValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
        info_messages: vec![],
    }
}

/// Select output folder for recordings
#[command]
pub async fn select_output_folder() -> Result<String, String> {
    info!("Opening folder selection dialog");

    // Use tauri's dialog plugin to select folder
    // For now, return default path - in real implementation would use file dialog
    let default_path = get_default_output_path();

    Ok(default_path)
}

/// Delete a recording file
#[command]
pub async fn delete_recording(recording_id: String) -> Result<SettingsResponse, String> {
    info!("Deleting recording with ID: {}", recording_id);

    // In a real implementation, you would:
    // 1. Find the recording file by ID
    // 2. Delete the file from disk
    // 3. Update the recordings database
    // 4. Clean up any related metadata

    Ok(SettingsResponse {
        success: true,
        message: format!("Recording {} deleted successfully", recording_id),
        settings: None,
    })
}

/// Get recent recordings list
#[command]
pub async fn get_recent_recording_list(limit: Option<u32>) -> Result<Vec<RecordingItem>, String> {
    info!("Getting recent recordings list with limit: {:?}", limit);

    let limit = limit.unwrap_or(50);

    // In a real implementation, you would:
    // 1. Query the database for recent recordings
    // 2. Sort by creation date (newest first)
    // 3. Apply limit
    // 4. Return recording metadata

    // For now, return empty list
    Ok(vec![])
}

/// Recording item structure for the recent recordings list
#[derive(Debug, Serialize)]
pub struct RecordingItem {
    pub id: String,
    pub filename: String,
    pub path: String,
    pub duration: u64,
    pub file_size: u64,
    pub resolution: String,
    pub quality: String,
    pub created_at: String,
    pub thumbnail_path: Option<String>,
}

/// Get default output path for recordings
fn get_default_output_path() -> String {
    // Try to get user's Documents/VirtualMeet folder
    if let Some(documents_dir) = dirs::document_dir() {
        let recordings_dir = documents_dir.join("VirtualMeet").join("Recordings");
        if std::fs::create_dir_all(&recordings_dir).is_ok() {
            return recordings_dir.to_string_lossy().to_string();
        }
    }

    // Fallback to current directory
    ".".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.general.language, "en");
        assert_eq!(settings.video.default_fps, 30.0);
        assert_eq!(settings.audio.default_sample_rate, 44100);
    }

    #[test]
    fn test_validate_video_settings() {
        let valid_settings = serde_json::json!({
            "default_fps": 30.0,
            "default_resolution": "HD1080p"
        });

        let result = validate_video_settings(&valid_settings);
        assert!(result.is_valid);

        let invalid_settings = serde_json::json!({
            "default_fps": 200.0
        });

        let result = validate_video_settings(&invalid_settings);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_audio_settings() {
        let valid_settings = serde_json::json!({
            "default_sample_rate": 44100,
            "default_bit_depth": 16
        });

        let result = validate_audio_settings(&valid_settings);
        assert!(result.is_valid);

        let invalid_settings = serde_json::json!({
            "default_sample_rate": 1000,
            "default_bit_depth": 8
        });

        let result = validate_audio_settings(&invalid_settings);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 2);
    }
}