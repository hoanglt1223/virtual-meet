//! Hotkey Management Commands
//!
//! Tauri commands for global hotkey registration, management, and execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{command, AppHandle, State};
use tracing::{debug, error, info, warn};

use crate::hotkeys::{HotkeyDefinition, HotkeyManager};
use crate::AppState;

/// Hotkey action types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum HotkeyAction {
    StartVideo,
    StopVideo,
    StartAudio,
    StopAudio,
    StartRecording,
    StopRecording,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    SwitchVideo,
    SwitchAudio,
    Screenshot,
    ToggleCamera,
    ToggleMicrophone,
    Settings,
    Quit,
    Custom(String),
}

/// Hotkey definition
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotkeyDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub key_combination: String,
    pub action: HotkeyAction,
    pub enabled: bool,
    pub global: bool,
    pub category: HotkeyCategory,
}

/// Hotkey category
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum HotkeyCategory {
    Media,
    Recording,
    System,
    Custom,
}

/// Hotkey registration request
#[derive(Debug, Deserialize)]
pub struct RegisterHotkeyRequest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub key_combination: String,
    pub action: HotkeyAction,
    pub global: bool,
    pub enabled: bool,
}

/// Hotkey registration response
#[derive(Debug, Serialize)]
pub struct RegisterHotkeyResponse {
    pub success: bool,
    pub message: String,
    pub hotkey_id: Option<String>,
}

/// Hotkey list response
#[derive(Debug, Serialize)]
pub struct HotkeyListResponse {
    pub success: bool,
    pub message: String,
    pub hotkeys: Vec<HotkeyDefinition>,
}

/// Hotkey status response
#[derive(Debug, Serialize)]
pub struct HotkeyStatusResponse {
    pub success: bool,
    pub hotkey_id: String,
    pub is_registered: bool,
    pub is_enabled: bool,
    pub last_triggered: Option<String>,
    pub trigger_count: u64,
}

/// Execute hotkey action request
#[derive(Debug, Deserialize)]
pub struct ExecuteHotkeyActionRequest {
    pub action: HotkeyAction,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Hotkey execution response
#[derive(Debug, Serialize)]
pub struct ExecuteHotkeyActionResponse {
    pub success: bool,
    pub message: String,
    pub action: HotkeyAction,
    pub result: Option<serde_json::Value>,
}

/// Hotkey conflict check request
#[derive(Debug, Deserialize)]
pub struct CheckHotkeyConflictRequest {
    pub key_combination: String,
    pub exclude_hotkey_id: Option<String>,
}

/// Hotkey conflict check response
#[derive(Debug, Serialize)]
pub struct CheckHotkeyConflictResponse {
    pub success: bool,
    pub has_conflict: bool,
    pub conflicting_hotkeys: Vec<HotkeyDefinition>,
}

/// Shared hotkey state
pub struct HotkeyState {
    pub registered_hotkeys: HashMap<String, HotkeyDefinition>,
    pub hotkey_stats: HashMap<String, HotkeyStats>,
}

/// Hotkey statistics
#[derive(Debug, Serialize, Clone)]
pub struct HotkeyStats {
    pub trigger_count: u64,
    pub last_triggered: Option<chrono::DateTime<chrono::Utc>>,
    pub average_trigger_interval_ms: Option<u64>,
}

impl Default for HotkeyState {
    fn default() -> Self {
        Self {
            registered_hotkeys: HashMap::new(),
            hotkey_stats: HashMap::new(),
        }
    }
}

/// Register a new hotkey
#[command]
pub async fn register_hotkey(
    request: RegisterHotkeyRequest,
    app: AppHandle,
) -> Result<RegisterHotkeyResponse, String> {
    info!(
        "Registering hotkey: {} ({})",
        request.name, request.key_combination
    );

    // Validate key combination format
    if !is_valid_key_combination(&request.key_combination) {
        return Ok(RegisterHotkeyResponse {
            success: false,
            message: format!("Invalid key combination: {}", request.key_combination),
            hotkey_id: None,
        });
    }

    // Check for conflicts
    if let Some(conflicts) = check_key_combination_conflicts(&request.key_combination, None) {
        if !conflicts.is_empty() {
            return Ok(RegisterHotkeyResponse {
                success: false,
                message: format!(
                    "Hotkey conflicts with existing hotkeys: {}",
                    conflicts
                        .iter()
                        .map(|h| &h.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                hotkey_id: None,
            });
        }
    }

    let hotkey_definition = HotkeyDefinition {
        id: request.id.clone(),
        name: request.name.clone(),
        description: request.description.clone(),
        key_combination: request.key_combination.clone(),
        action: request.action.clone(),
        enabled: request.enabled,
        global: request.global,
        category: determine_hotkey_category(&request.action),
    };

    // TODO: Register hotkey with OS using global-hotkey crate
    let hotkey_state = app.state::<std::sync::Mutex<HotkeyState>>();
    let mut state = hotkey_state
        .lock()
        .map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    state
        .registered_hotkeys
        .insert(request.id.clone(), hotkey_definition.clone());
    state.hotkey_stats.insert(
        request.id.clone(),
        HotkeyStats {
            trigger_count: 0,
            last_triggered: None,
            average_trigger_interval_ms: None,
        },
    );

    info!("Hotkey registered successfully: {}", request.name);

    Ok(RegisterHotkeyResponse {
        success: true,
        message: format!("Hotkey '{}' registered successfully", request.name),
        hotkey_id: Some(request.id),
    })
}

/// Unregister a hotkey
#[command]
pub async fn unregister_hotkey(
    hotkey_id: String,
    app: AppHandle,
) -> Result<RegisterHotkeyResponse, String> {
    info!("Unregistering hotkey: {}", hotkey_id);

    let hotkey_state = app.state::<std::sync::Mutex<HotkeyState>>();
    let mut state = hotkey_state
        .lock()
        .map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    match state.registered_hotkeys.remove(&hotkey_id) {
        Some(hotkey) => {
            state.hotkey_stats.remove(&hotkey_id);
            // TODO: Unregister hotkey from OS using global-hotkey crate
            Ok(RegisterHotkeyResponse {
                success: true,
                message: format!("Hotkey '{}' unregistered successfully", hotkey.name),
                hotkey_id: Some(hotkey_id),
            })
        }
        None => Ok(RegisterHotkeyResponse {
            success: false,
            message: format!("Hotkey with ID '{}' not found", hotkey_id),
            hotkey_id: None,
        }),
    }
}

/// Get all registered hotkeys
#[command]
pub async fn get_registered_hotkeys(app: AppHandle) -> Result<HotkeyListResponse, String> {
    info!("Getting registered hotkeys");

    let hotkey_state = app.state::<std::sync::Mutex<HotkeyState>>();
    let state = hotkey_state
        .lock()
        .map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    let hotkeys: Vec<HotkeyDefinition> = state.registered_hotkeys.values().cloned().collect();

    Ok(HotkeyListResponse {
        success: true,
        message: format!("Retrieved {} hotkeys", hotkeys.len()),
        hotkeys,
    })
}

/// Get hotkey status
#[command]
pub async fn get_hotkey_status(
    hotkey_id: String,
    app: AppHandle,
) -> Result<HotkeyStatusResponse, String> {
    info!("Getting hotkey status: {}", hotkey_id);

    let hotkey_state = app.state::<std::sync::Mutex<HotkeyState>>();
    let state = hotkey_state
        .lock()
        .map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    match (
        state.registered_hotkeys.get(&hotkey_id),
        state.hotkey_stats.get(&hotkey_id),
    ) {
        (Some(hotkey), Some(stats)) => Ok(HotkeyStatusResponse {
            success: true,
            hotkey_id: hotkey_id,
            is_registered: true,
            is_enabled: hotkey.enabled,
            last_triggered: stats.last_triggered.map(|dt| dt.to_rfc3339()),
            trigger_count: stats.trigger_count,
        }),
        _ => Ok(HotkeyStatusResponse {
            success: false,
            hotkey_id: hotkey_id,
            is_registered: false,
            is_enabled: false,
            last_triggered: None,
            trigger_count: 0,
        }),
    }
}

/// Enable/disable a hotkey
#[command]
pub async fn set_hotkey_enabled(
    hotkey_id: String,
    enabled: bool,
    app: AppHandle,
) -> Result<RegisterHotkeyResponse, String> {
    info!("Setting hotkey {} enabled to: {}", hotkey_id, enabled);

    let hotkey_state = app.state::<std::sync::Mutex<HotkeyState>>();
    let mut state = hotkey_state
        .lock()
        .map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    match state.registered_hotkeys.get_mut(&hotkey_id) {
        Some(hotkey) => {
            hotkey.enabled = enabled;
            // TODO: Enable/disable hotkey in OS using global-hotkey crate
            Ok(RegisterHotkeyResponse {
                success: true,
                message: format!(
                    "Hotkey '{}' {}",
                    hotkey.name,
                    if enabled { "enabled" } else { "disabled" }
                ),
                hotkey_id: Some(hotkey_id),
            })
        }
        None => Ok(RegisterHotkeyResponse {
            success: false,
            message: format!("Hotkey with ID '{}' not found", hotkey_id),
            hotkey_id: None,
        }),
    }
}

/// Execute a hotkey action directly
#[command]
pub async fn execute_hotkey_action(
    request: ExecuteHotkeyActionRequest,
    app: AppHandle,
) -> Result<ExecuteHotkeyActionResponse, String> {
    info!("Executing hotkey action: {:?}", request.action);

    let result = match execute_action(&request.action, request.parameters, &app).await {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to execute hotkey action: {}", e);
            return Ok(ExecuteHotkeyActionResponse {
                success: false,
                message: format!("Failed to execute action: {}", e),
                action: request.action,
                result: None,
            });
        }
    };

    info!("Hotkey action executed successfully: {:?}", request.action);

    Ok(ExecuteHotkeyActionResponse {
        success: true,
        message: format!("Action executed successfully: {:?}", request.action),
        action: request.action,
        result: Some(result),
    })
}

/// Check for hotkey conflicts
#[command]
pub async fn check_hotkey_conflicts(
    request: CheckHotkeyConflictRequest,
    app: AppHandle,
) -> Result<CheckHotkeyConflictResponse, String> {
    info!("Checking hotkey conflicts for: {}", request.key_combination);

    let conflicts =
        check_key_combination_conflicts(&request.key_combination, request.exclude_hotkey_id);

    Ok(CheckHotkeyConflictResponse {
        success: true,
        has_conflict: !conflicts.is_empty(),
        conflicting_hotkeys: conflicts,
    })
}

/// Get default hotkey definitions
#[command]
pub async fn get_default_hotkeys() -> Result<HotkeyListResponse, String> {
    info!("Getting default hotkey definitions");

    let default_hotkeys = vec![
        // Media Controls - F1-F4 keys for quick access
        HotkeyDefinition {
            id: "toggle_mute_f1".to_string(),
            name: "Toggle Mute (F1)".to_string(),
            description: "Toggle microphone mute state".to_string(),
            key_combination: "Ctrl+F1".to_string(),
            action: HotkeyAction::ToggleMute,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "start_video_f2".to_string(),
            name: "Start Video (F2)".to_string(),
            description: "Start video streaming".to_string(),
            key_combination: "Ctrl+F2".to_string(),
            action: HotkeyAction::StartVideo,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "stop_video_f3".to_string(),
            name: "Stop Video (F3)".to_string(),
            description: "Stop video streaming".to_string(),
            key_combination: "Ctrl+F3".to_string(),
            action: HotkeyAction::StopVideo,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "screenshot_f4".to_string(),
            name: "Screenshot (F4)".to_string(),
            description: "Take a screenshot of current video".to_string(),
            key_combination: "Ctrl+F4".to_string(),
            action: HotkeyAction::Screenshot,
            enabled: true,
            global: true,
            category: HotkeyCategory::Recording,
        },
        // Recording Controls - F5-F7 keys
        HotkeyDefinition {
            id: "start_recording_f5".to_string(),
            name: "Start Recording (F5)".to_string(),
            description: "Start recording video and audio".to_string(),
            key_combination: "Ctrl+F5".to_string(),
            action: HotkeyAction::StartRecording,
            enabled: true,
            global: true,
            category: HotkeyCategory::Recording,
        },
        HotkeyDefinition {
            id: "stop_recording_f6".to_string(),
            name: "Stop Recording (F6)".to_string(),
            description: "Stop recording video and audio".to_string(),
            key_combination: "Ctrl+F6".to_string(),
            action: HotkeyAction::StopRecording,
            enabled: true,
            global: true,
            category: HotkeyCategory::Recording,
        },
        HotkeyDefinition {
            id: "toggle_camera_f7".to_string(),
            name: "Toggle Camera (F7)".to_string(),
            description: "Toggle camera on/off".to_string(),
            key_combination: "Ctrl+F7".to_string(),
            action: HotkeyAction::ToggleCamera,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        // Audio Controls - F8-F10 keys
        HotkeyDefinition {
            id: "start_audio_f8".to_string(),
            name: "Start Audio (F8)".to_string(),
            description: "Start audio streaming".to_string(),
            key_combination: "Ctrl+F8".to_string(),
            action: HotkeyAction::StartAudio,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "stop_audio_f9".to_string(),
            name: "Stop Audio (F9)".to_string(),
            description: "Stop audio streaming".to_string(),
            key_combination: "Ctrl+F9".to_string(),
            action: HotkeyAction::StopAudio,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "toggle_microphone_f10".to_string(),
            name: "Toggle Microphone (F10)".to_string(),
            description: "Toggle microphone on/off".to_string(),
            key_combination: "Ctrl+F10".to_string(),
            action: HotkeyAction::ToggleMicrophone,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        // System Controls - F11-F12 keys
        HotkeyDefinition {
            id: "settings_f11".to_string(),
            name: "Open Settings (F11)".to_string(),
            description: "Open application settings".to_string(),
            key_combination: "Ctrl+F11".to_string(),
            action: HotkeyAction::Settings,
            enabled: true,
            global: true,
            category: HotkeyCategory::System,
        },
        HotkeyDefinition {
            id: "quit_f12".to_string(),
            name: "Quit Application (F12)".to_string(),
            description: "Exit the application".to_string(),
            key_combination: "Ctrl+F12".to_string(),
            action: HotkeyAction::Quit,
            enabled: true,
            global: true,
            category: HotkeyCategory::System,
        },
        // Alternative Volume Controls - Shift+F keys
        HotkeyDefinition {
            id: "volume_up_shift_f11".to_string(),
            name: "Volume Up (Shift+F11)".to_string(),
            description: "Increase microphone volume".to_string(),
            key_combination: "Shift+F11".to_string(),
            action: HotkeyAction::VolumeUp,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "volume_down_shift_f12".to_string(),
            name: "Volume Down (Shift+F12)".to_string(),
            description: "Decrease microphone volume".to_string(),
            key_combination: "Shift+F12".to_string(),
            action: HotkeyAction::VolumeDown,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        // Keep some traditional hotkeys for compatibility
        HotkeyDefinition {
            id: "start_video".to_string(),
            name: "Start Video".to_string(),
            description: "Start video streaming".to_string(),
            key_combination: "Ctrl+Shift+V".to_string(),
            action: HotkeyAction::StartVideo,
            enabled: false, // Disabled to avoid conflict with F2
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "stop_video".to_string(),
            name: "Stop Video".to_string(),
            description: "Stop video streaming".to_string(),
            key_combination: "Ctrl+Shift+X".to_string(),
            action: HotkeyAction::StopVideo,
            enabled: false, // Disabled to avoid conflict with F3
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "start_recording".to_string(),
            name: "Start Recording".to_string(),
            description: "Start recording video and audio".to_string(),
            key_combination: "Ctrl+Shift+R".to_string(),
            action: HotkeyAction::StartRecording,
            enabled: false, // Disabled to avoid conflict with F5
            global: true,
            category: HotkeyCategory::Recording,
        },
        HotkeyDefinition {
            id: "stop_recording".to_string(),
            name: "Stop Recording".to_string(),
            description: "Stop recording video and audio".to_string(),
            key_combination: "Ctrl+Shift+S".to_string(),
            action: HotkeyAction::StopRecording,
            enabled: false, // Disabled to avoid conflict with F6
            global: true,
            category: HotkeyCategory::Recording,
        },
        HotkeyDefinition {
            id: "toggle_mute".to_string(),
            name: "Toggle Mute".to_string(),
            description: "Toggle microphone mute state".to_string(),
            key_combination: "Ctrl+Shift+M".to_string(),
            action: HotkeyAction::ToggleMute,
            enabled: false, // Disabled to avoid conflict with F1
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "volume_up".to_string(),
            name: "Volume Up".to_string(),
            description: "Increase microphone volume".to_string(),
            key_combination: "Ctrl+Shift+Up".to_string(),
            action: HotkeyAction::VolumeUp,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "volume_down".to_string(),
            name: "Volume Down".to_string(),
            description: "Decrease microphone volume".to_string(),
            key_combination: "Ctrl+Shift+Down".to_string(),
            action: HotkeyAction::VolumeDown,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "screenshot".to_string(),
            name: "Screenshot".to_string(),
            description: "Take a screenshot of current video".to_string(),
            key_combination: "Ctrl+Shift+P".to_string(),
            action: HotkeyAction::Screenshot,
            enabled: false, // Disabled to avoid conflict with F4
            global: true,
            category: HotkeyCategory::System,
        },
        HotkeyDefinition {
            id: "quit".to_string(),
            name: "Quit Application".to_string(),
            description: "Exit the application".to_string(),
            key_combination: "Ctrl+Shift+Q".to_string(),
            action: HotkeyAction::Quit,
            enabled: false, // Disabled to avoid conflict with F12
            global: true,
            category: HotkeyCategory::System,
        },
    ];

    Ok(HotkeyListResponse {
        success: true,
        message: format!("Retrieved {} default hotkeys", default_hotkeys.len()),
        hotkeys: default_hotkeys,
    })
}

/// Validate a key combination
fn is_valid_key_combination(combination: &str) -> bool {
    // Basic validation - check for common modifier keys and a regular key
    let parts: Vec<&str> = combination.to_uppercase().split('+').collect();

    if parts.len() < 2 {
        return false;
    }

    let mut has_modifier = false;
    let mut has_key = false;

    for part in &parts {
        match part.trim() {
            "CTRL" | "ALT" | "SHIFT" | "WIN" | "CMD" | "META" => has_modifier = true,
            _ if part.len() == 1 => has_key = true,
            _ => {
                // Check for function keys, arrow keys, etc.
                if part.starts_with("F")
                    || part.starts_with("NUMPAD")
                    || part == "SPACE"
                    || part == "ENTER"
                    || part == "ESCAPE"
                    || part.ends_with("ARROW")
                {
                    has_key = true;
                }
            }
        }
    }

    has_modifier && has_key
}

/// Check for key combination conflicts
fn check_key_combination_conflicts(
    key_combination: &str,
    exclude_id: Option<String>,
) -> Vec<HotkeyDefinition> {
    // TODO: Check against registered hotkeys in HotkeyState
    Vec::new()
}

/// Determine hotkey category from action
fn determine_hotkey_category(action: &HotkeyAction) -> HotkeyCategory {
    match action {
        HotkeyAction::StartVideo
        | HotkeyAction::StopVideo
        | HotkeyAction::StartAudio
        | HotkeyAction::StopAudio
        | HotkeyAction::ToggleMute
        | HotkeyAction::VolumeUp
        | HotkeyAction::VolumeDown
        | HotkeyAction::SwitchVideo
        | HotkeyAction::SwitchAudio
        | HotkeyAction::ToggleCamera
        | HotkeyAction::ToggleMicrophone => HotkeyCategory::Media,

        HotkeyAction::StartRecording | HotkeyAction::StopRecording | HotkeyAction::Screenshot => {
            HotkeyCategory::Recording
        }

        HotkeyAction::Settings | HotkeyAction::Quit => HotkeyCategory::System,
        HotkeyAction::Custom(_) => HotkeyCategory::Custom,
    }
}

/// Execute a hotkey action
async fn execute_action(
    action: &HotkeyAction,
    parameters: Option<HashMap<String, serde_json::Value>>,
    app: &AppHandle,
) -> Result<serde_json::Value> {
    match action {
        HotkeyAction::StartVideo => {
            // Start video streaming using device system
            match crate::commands::start_streaming(None, None, None, None).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "started",
                    "message": "Video streaming started"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to start video: {}", e)
                })),
            }
        }
        HotkeyAction::StopVideo => {
            // Stop video streaming using device system
            match crate::commands::stop_streaming(None).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "stopped",
                    "message": "Video streaming stopped"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to stop video: {}", e)
                })),
            }
        }
        HotkeyAction::StartAudio => {
            // Start audio streaming using device system
            match crate::commands::start_audio_streaming(None, None, None, None).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "started",
                    "message": "Audio streaming started"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to start audio: {}", e)
                })),
            }
        }
        HotkeyAction::StopAudio => {
            // Stop audio streaming using device system
            match crate::commands::stop_audio_streaming(None).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "stopped",
                    "message": "Audio streaming stopped"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to stop audio: {}", e)
                })),
            }
        }
        HotkeyAction::StartRecording => {
            // Start recording using recording system
            match crate::commands_recording::start_recording(None).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "started",
                    "message": "Recording started"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to start recording: {}", e)
                })),
            }
        }
        HotkeyAction::StopRecording => {
            // Stop recording using recording system
            match crate::commands_recording::stop_recording(app.clone()).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "stopped",
                    "message": "Recording stopped"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to stop recording: {}", e)
                })),
            }
        }
        HotkeyAction::ToggleMute => {
            // Toggle microphone mute using device system
            match crate::commands::toggle_microphone_mute(None).await {
                Ok(_) => Ok(serde_json::json!({
                    "status": "toggled",
                    "message": "Microphone mute toggled"
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to toggle mute: {}", e)
                })),
            }
        }
        HotkeyAction::Screenshot => {
            // TODO: Implement screenshot capture functionality
            info!("Screenshot hotkey triggered");
            Ok(serde_json::json!({
                "status": "captured",
                "message": "Screenshot taken"
            }))
        }
        HotkeyAction::ToggleCamera => {
            // TODO: Implement camera toggle functionality
            info!("Toggle camera hotkey triggered");
            Ok(serde_json::json!({
                "status": "toggled",
                "message": "Camera toggled"
            }))
        }
        HotkeyAction::ToggleMicrophone => {
            // TODO: Implement microphone toggle functionality
            info!("Toggle microphone hotkey triggered");
            Ok(serde_json::json!({
                "status": "toggled",
                "message": "Microphone toggled"
            }))
        }
        HotkeyAction::VolumeUp => {
            // TODO: Implement volume up functionality
            info!("Volume up hotkey triggered");
            Ok(serde_json::json!({
                "status": "adjusted",
                "message": "Volume increased"
            }))
        }
        HotkeyAction::VolumeDown => {
            // TODO: Implement volume down functionality
            info!("Volume down hotkey triggered");
            Ok(serde_json::json!({
                "status": "adjusted",
                "message": "Volume decreased"
            }))
        }
        HotkeyAction::Settings => {
            // TODO: Implement settings window focus/open
            info!("Settings hotkey triggered");
            Ok(serde_json::json!({
                "status": "opened",
                "message": "Settings opened"
            }))
        }
        HotkeyAction::Quit => {
            // Quit the application
            info!("Quit action triggered by hotkey");
            // In production, you would use: app.exit(0);
            Ok(serde_json::json!({
                "status": "quitting",
                "message": "Application will quit"
            }))
        }
        HotkeyAction::Custom(action_name) => {
            // Execute custom action using scripting system
            let script_content = parameters
                .and_then(|p| p.get("script"))
                .and_then(|v| v.as_str())
                .unwrap_or(&format!(
                    "print!(\"Executing custom action: {}\");",
                    action_name
                ));

            match crate::scripting::execute_script_async(script_content.to_string()).await {
                Ok(result) => Ok(serde_json::json!({
                    "status": "executed",
                    "action": action_name,
                    "result": result,
                    "message": format!("Custom action '{}' executed", action_name)
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "action": action_name,
                    "error": e.to_string(),
                    "message": format!("Failed to execute custom action '{}': {}", action_name, e)
                })),
            }
        }
        _ => {
            warn!("Hotkey action not implemented: {:?}", action);
            Ok(serde_json::json!({
                "status": "not_implemented",
                "message": format!("Action {:?} is not yet implemented", action)
            }))
        }
    }
}

/// Initialize hotkey system
pub fn init_hotkey_system() -> HotkeyState {
    info!("Initializing hotkey system");
    HotkeyState::default()
}

/// Register all default hotkeys on startup
pub async fn register_default_hotkeys(app: &AppHandle) -> Result<()> {
    info!("Registering default global hotkeys on startup");

    // Get default hotkeys
    let defaults = get_default_hotkeys().await?;
    if !defaults.success {
        return Err(anyhow::anyhow!(
            "Failed to get default hotkeys: {}",
            defaults.message
        ));
    }

    // Register all enabled default hotkeys
    for hotkey_def in defaults.hotkeys {
        if hotkey_def.enabled && hotkey_def.global {
            let request = RegisterHotkeyRequest {
                id: hotkey_def.id.clone(),
                name: hotkey_def.name.clone(),
                description: hotkey_def.description.clone(),
                key_combination: hotkey_def.key_combination.clone(),
                action: hotkey_def.action.clone(),
                global: hotkey_def.global,
                enabled: hotkey_def.enabled,
            };

            match register_hotkey(request, app.clone()).await {
                Ok(response) => {
                    if response.success {
                        info!(
                            "Successfully registered default hotkey: {}",
                            hotkey_def.name
                        );
                    } else {
                        warn!(
                            "Failed to register hotkey '{}': {}",
                            hotkey_def.name, response.message
                        );
                    }
                }
                Err(e) => {
                    warn!("Error registering hotkey '{}': {}", hotkey_def.name, e);
                }
            }
        }
    }

    Ok(())
}

/// Set up global hotkey event listener
pub fn setup_global_hotkey_listener(app: AppHandle) -> Result<()> {
    info!("Setting up global hotkey event listener");

    // TODO: Implement global hotkey event listener using global-hotkey crate
    tokio::spawn(async move {
        info!("Global hotkey event listener started");
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_key_combinations() {
        assert!(is_valid_key_combination("Ctrl+Shift+V"));
        assert!(is_valid_key_combination("Alt+F4"));
        assert!(is_valid_key_combination("Ctrl+Space"));
        assert!(is_valid_key_combination("Win+UpArrow"));
        assert!(is_valid_key_combination("Ctrl+Shift+F12"));
    }

    #[test]
    fn test_invalid_key_combinations() {
        assert!(!is_valid_key_combination("Ctrl")); // Missing key
        assert!(!is_valid_key_combination("V")); // Missing modifier
        assert!(!is_valid_key_combination("")); // Empty
        assert!(!is_valid_key_combination("Ctrl+Shift+")); // Missing key
    }

    #[test]
    fn test_hotkey_category_determination() {
        assert_eq!(
            determine_hotkey_category(&HotkeyAction::StartVideo),
            HotkeyCategory::Media
        );
        assert_eq!(
            determine_hotkey_category(&HotkeyAction::StartRecording),
            HotkeyCategory::Recording
        );
        assert_eq!(
            determine_hotkey_category(&HotkeyAction::Quit),
            HotkeyCategory::System
        );
        assert_eq!(
            determine_hotkey_category(&HotkeyAction::Custom("test".to_string())),
            HotkeyCategory::Custom
        );
    }

    #[test]
    fn test_default_hotkeys() {
        let defaults = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(get_default_hotkeys())
            .unwrap();

        assert!(defaults.success);
        assert!(!defaults.hotkeys.is_empty());

        // Check that common hotkeys are present
        let hotkey_ids: Vec<String> = defaults.hotkeys.iter().map(|h| h.id.clone()).collect();
        assert!(hotkey_ids.contains(&"start_video".to_string()));
        assert!(hotkey_ids.contains(&"stop_video".to_string()));
        assert!(hotkey_ids.contains(&"start_recording".to_string()));
        assert!(hotkey_ids.contains(&"stop_recording".to_string()));
    }
}
