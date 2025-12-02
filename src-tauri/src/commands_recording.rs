//! Recording Commands
//!
//! Tauri commands for the combined recording functionality.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Manager, Runtime};
use tracing::{error, info, warn};

use crate::recording::{
    AudioCodec, AudioQualityPreset, CombinedRecorder, FrameRate, OutputFormat, RecordingConfig,
    RecordingState, RecordingStats, VideoCodec, VideoQualityPreset, VideoResolution,
};

/// Start recording command
#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    output_path: String,
    config: Option<RecordingConfigOptions>,
) -> Result<String, String> {
    info!("Starting recording to: {}", output_path);

    // Get or create recorder instance
    let recorder = app.state::<std::sync::Mutex<CombinedRecorder>>();
    let mut recorder = recorder
        .lock()
        .map_err(|e| format!("Failed to lock recorder: {}", e))?;

    // Convert config options to full config
    let recording_config = if let Some(options) = config {
        options.to_recording_config()?
    } else {
        RecordingConfig::hd_1080p()
    };

    // Start recording
    let session_id = recorder
        .start_recording(&output_path)
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    info!("Recording started with session ID: {}", session_id);
    Ok(session_id)
}

/// Stop recording command
#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<(), String> {
    info!("Stopping recording");

    // Get recorder instance
    let recorder = app.state::<std::sync::Mutex<CombinedRecorder>>();
    let mut recorder = recorder
        .lock()
        .map_err(|e| format!("Failed to lock recorder: {}", e))?;

    // Stop recording
    recorder
        .stop_recording()
        .map_err(|e| format!("Failed to stop recording: {}", e))?;

    info!("Recording stopped");
    Ok(())
}

/// Get recording status command
#[tauri::command]
pub async fn get_recording_status(
    app: tauri::AppHandle,
) -> Result<RecordingStatusResponse, String> {
    // Get recorder instance
    let recorder = app.state::<std::sync::Mutex<CombinedRecorder>>();
    let recorder = recorder
        .lock()
        .map_err(|e| format!("Failed to lock recorder: {}", e))?;

    // Get current state and stats
    let state = recorder
        .get_state()
        .map_err(|e| format!("Failed to get state: {}", e))?;
    let stats = recorder
        .get_stats()
        .map_err(|e| format!("Failed to get stats: {}", e))?;
    let current_session = recorder
        .get_current_session()
        .map_err(|e| format!("Failed to get session: {}", e))?;

    Ok(RecordingStatusResponse {
        state,
        stats,
        current_session,
    })
}

/// Update recording configuration command
#[tauri::command]
pub async fn update_recording_config(
    app: tauri::AppHandle,
    config: RecordingConfigOptions,
) -> Result<(), String> {
    info!("Updating recording configuration");

    // Get recorder instance
    let recorder = app.state::<std::sync::Mutex<CombinedRecorder>>();
    let mut recorder = recorder
        .lock()
        .map_err(|e| format!("Failed to lock recorder: {}", e))?;

    // Convert options to full config
    let recording_config = config.to_recording_config()?;

    // Update configuration
    recorder
        .update_config(recording_config)
        .map_err(|e| format!("Failed to update config: {}", e))?;

    info!("Recording configuration updated");
    Ok(())
}

/// Get available recording presets command
#[tauri::command]
pub async fn get_recording_presets() -> Result<Vec<RecordingPreset>, String> {
    let presets = vec![
        RecordingPreset {
            id: "fast_720p".to_string(),
            name: "Fast 720p".to_string(),
            description: "Low quality, 720p for fast recording".to_string(),
            config: RecordingConfig::fast_recording(),
        },
        RecordingPreset {
            id: "balanced_1080p".to_string(),
            name: "Balanced 1080p".to_string(),
            description: "Good quality, 1080p balanced performance".to_string(),
            config: RecordingConfig::hd_1080p(),
        },
        RecordingPreset {
            id: "high_1080p".to_string(),
            name: "High Quality 1080p".to_string(),
            description: "High quality, 1080p recording".to_string(),
            config: RecordingConfig::high_quality(),
        },
        RecordingPreset {
            id: "custom_720p".to_string(),
            name: "Custom 720p".to_string(),
            description: "Custom configuration for 720p recording".to_string(),
            config: RecordingConfig::hd_720p(),
        },
    ];

    Ok(presets)
}

/// Test recording capabilities command
#[tauri::command]
pub async fn test_recording_capabilities(
    app: tauri::AppHandle,
) -> Result<RecordingCapabilities, String> {
    // This would test system capabilities for recording
    // For now, return a basic capability assessment

    let capabilities = RecordingCapabilities {
        can_record_video: true,
        can_record_audio: true,
        supported_resolutions: vec![VideoResolution::HD720p, VideoResolution::HD1080p],
        supported_video_codecs: vec![VideoCodec::H264],
        supported_audio_codecs: vec![AudioCodec::AAC],
        max_concurrent_recordings: 1,
        estimated_max_bitrate: 10_000_000, // 10 Mbps
        hardware_acceleration_available: false,
        recommended_presets: vec!["fast_720p".to_string(), "balanced_1080p".to_string()],
    };

    Ok(capabilities)
}

/// Initialize recorder command
pub async fn initialize_recorder<R: Runtime>(app: &mut tauri::App<R>) -> Result<()> {
    info!("Initializing combined recorder");

    // Create default recorder configuration
    let config = RecordingConfig::default();

    // Create recorder instance
    let recorder = CombinedRecorder::new(config)
        .map_err(|e| anyhow::anyhow!("Failed to create recorder: {}", e))?;

    // Store recorder in app state
    app.manage(std::sync::Mutex::new(recorder));

    info!("Combined recorder initialized successfully");
    Ok(())
}

/// Recording configuration options (simplified for command interface)
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RecordingConfigOptions {
    pub video_resolution: Option<String>,
    pub video_quality: Option<String>,
    pub video_codec: Option<String>,
    pub frame_rate: Option<f32>,
    pub video_bitrate: Option<u32>,
    pub audio_quality: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_bitrate: Option<u32>,
    pub output_format: Option<String>,
}

impl RecordingConfigOptions {
    fn to_recording_config(self) -> Result<RecordingConfig> {
        let mut config = RecordingConfig::default();

        // Video settings
        if let Some(resolution_str) = self.video_resolution {
            config.video.resolution = match resolution_str.as_str() {
                "720p" => VideoResolution::HD720p,
                "1080p" => VideoResolution::HD1080p,
                "480p" => VideoResolution::WVGA,
                _ => VideoResolution::HD1080p, // Default
            };
        }

        if let Some(quality_str) = self.video_quality {
            config.video.quality_preset = match quality_str.as_str() {
                "fast" => VideoQualityPreset::Fast,
                "high" => VideoQualityPreset::High,
                "ultra" => VideoQualityPreset::Ultra,
                _ => VideoQualityPreset::Balanced, // Default
            };
        }

        if let Some(codec_str) = self.video_codec {
            config.video.codec = match codec_str.as_str() {
                "h265" => VideoCodec::H265,
                "vp9" => VideoCodec::VP9,
                _ => VideoCodec::H264, // Default
            };
        }

        if let Some(fps) = self.frame_rate {
            config.video.frame_rate = FrameRate::Custom(fps);
        }

        if let Some(bitrate) = self.video_bitrate {
            config.video.target_bitrate = bitrate;
        }

        // Audio settings
        if let Some(quality_str) = self.audio_quality {
            config.audio.quality_preset = match quality_str.as_str() {
                "low" => AudioQualityPreset::Low,
                "voice" => AudioQualityPreset::Voice,
                "high" => AudioQualityPreset::High,
                _ => AudioQualityPreset::Standard, // Default
            };
        }

        if let Some(codec_str) = self.audio_codec {
            config.audio.codec = match codec_str.as_str() {
                "opus" => AudioCodec::Opus,
                "mp3" => AudioCodec::MP3,
                _ => AudioCodec::AAC, // Default
            };
        }

        if let Some(bitrate) = self.audio_bitrate {
            config.audio.target_bitrate = bitrate;
        }

        // Output settings
        if let Some(format_str) = self.output_format {
            config.output.format = match format_str.as_str() {
                "mkv" => OutputFormat::MKV,
                "webm" => OutputFormat::WebM,
                _ => OutputFormat::MP4, // Default
            };
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }
}

/// Recording status response
#[derive(Debug, Serialize)]
pub struct RecordingStatusResponse {
    pub state: RecordingState,
    pub stats: RecordingStats,
    pub current_session: Option<crate::recording::RecordingSession>,
}

/// Recording preset
#[derive(Debug, Serialize)]
pub struct RecordingPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: RecordingConfig,
}

/// Recording capabilities
#[derive(Debug, Serialize)]
pub struct RecordingCapabilities {
    pub can_record_video: bool,
    pub can_record_audio: bool,
    pub supported_resolutions: Vec<VideoResolution>,
    pub supported_video_codecs: Vec<VideoCodec>,
    pub supported_audio_codecs: Vec<AudioCodec>,
    pub max_concurrent_recordings: u32,
    pub estimated_max_bitrate: u32,
    pub hardware_acceleration_available: bool,
    pub recommended_presets: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_config_options() {
        let options = RecordingConfigOptions {
            video_resolution: Some("1080p".to_string()),
            video_quality: Some("high".to_string()),
            frame_rate: Some(60.0),
            audio_bitrate: Some(192000),
            ..Default::default()
        };

        let config = options.to_recording_config().unwrap();
        assert_eq!(config.video.resolution, VideoResolution::HD1080p);
        assert_eq!(config.video.quality_preset, VideoQualityPreset::High);
        assert_eq!(config.video.frame_rate, FrameRate::Custom(60.0));
        assert_eq!(config.audio.target_bitrate, 192000);
    }

    #[test]
    fn test_recording_capabilities() {
        let capabilities = RecordingCapabilities {
            can_record_video: true,
            can_record_audio: true,
            supported_resolutions: vec![VideoResolution::HD1080p],
            supported_video_codecs: vec![VideoCodec::H264],
            supported_audio_codecs: vec![AudioCodec::AAC],
            max_concurrent_recordings: 1,
            estimated_max_bitrate: 10_000_000,
            hardware_acceleration_available: false,
            recommended_presets: vec!["balanced_1080p".to_string()],
        };

        assert!(capabilities.can_record_video);
        assert!(capabilities.can_record_audio);
        assert_eq!(capabilities.max_concurrent_recordings, 1);
    }
}
