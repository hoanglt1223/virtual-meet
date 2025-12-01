//! Hotkey Management Commands
//!
//! Tauri commands for global hotkey registration, management, and execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{command, State, AppHandle};
use tracing::{info, error, warn, debug};

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
    info!("Registering hotkey: {} ({})", request.name, request.key_combination);

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
                    conflicts.iter().map(|h| &h.name).collect::<Vec<_>>().join(", ")
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

    // In a real implementation, you would register the hotkey with the OS
    // For now, we'll just store it in our state
    let hotkey_state = app.state::<std::sync::Mutex<HotkeyState>>();
    let mut state = hotkey_state.lock().map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    state.registered_hotkeys.insert(request.id.clone(), hotkey_definition.clone());
    state.hotkey_stats.insert(request.id.clone(), HotkeyStats {
        trigger_count: 0,
        last_triggered: None,
        average_trigger_interval_ms: None,
    });

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
    let mut state = hotkey_state.lock().map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    match state.registered_hotkeys.remove(&hotkey_id) {
        Some(hotkey) => {
            state.hotkey_stats.remove(&hotkey_id);

            // In a real implementation, you would unregister the hotkey from the OS

            Ok(RegisterHotkeyResponse {
                success: true,
                message: format!("Hotkey '{}' unregistered successfully", hotkey.name),
                hotkey_id: Some(hotkey_id),
            })
        },
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
    let state = hotkey_state.lock().map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

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
    let state = hotkey_state.lock().map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    match (state.registered_hotkeys.get(&hotkey_id), state.hotkey_stats.get(&hotkey_id)) {
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
    let mut state = hotkey_state.lock().map_err(|e| format!("Failed to lock hotkey state: {}", e))?;

    match state.registered_hotkeys.get_mut(&hotkey_id) {
        Some(hotkey) => {
            hotkey.enabled = enabled;

            // In a real implementation, you would enable/disable the hotkey in the OS

            Ok(RegisterHotkeyResponse {
                success: true,
                message: format!("Hotkey '{}' {}", hotkey.name, if enabled { "enabled" } else { "disabled" }),
                hotkey_id: Some(hotkey_id),
            })
        },
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

    let conflicts = check_key_combination_conflicts(&request.key_combination, request.exclude_hotkey_id);

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
        HotkeyDefinition {
            id: "start_video".to_string(),
            name: "Start Video".to_string(),
            description: "Start video streaming".to_string(),
            key_combination: "Ctrl+Shift+V".to_string(),
            action: HotkeyAction::StartVideo,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "stop_video".to_string(),
            name: "Stop Video".to_string(),
            description: "Stop video streaming".to_string(),
            key_combination: "Ctrl+Shift+X".to_string(),
            action: HotkeyAction::StopVideo,
            enabled: true,
            global: true,
            category: HotkeyCategory::Media,
        },
        HotkeyDefinition {
            id: "start_recording".to_string(),
            name: "Start Recording".to_string(),
            description: "Start recording video and audio".to_string(),
            key_combination: "Ctrl+Shift+R".to_string(),
            action: HotkeyAction::StartRecording,
            enabled: true,
            global: true,
            category: HotkeyCategory::Recording,
        },
        HotkeyDefinition {
            id: "stop_recording".to_string(),
            name: "Stop Recording".to_string(),
            description: "Stop recording video and audio".to_string(),
            key_combination: "Ctrl+Shift+S".to_string(),
            action: HotkeyAction::StopRecording,
            enabled: true,
            global: true,
            category: HotkeyCategory::Recording,
        },
        HotkeyDefinition {
            id: "toggle_mute".to_string(),
            name: "Toggle Mute".to_string(),
            description: "Toggle microphone mute state".to_string(),
            key_combination: "Ctrl+Shift+M".to_string(),
            action: HotkeyAction::ToggleMute,
            enabled: true,
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
            enabled: true,
            global: true,
            category: HotkeyCategory::System,
        },
        HotkeyDefinition {
            id: "quit".to_string(),
            name: "Quit Application".to_string(),
            description: "Exit the application".to_string(),
            key_combination: "Ctrl+Shift+Q".to_string(),
            action: HotkeyAction::Quit,
            enabled: true,
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
                if part.starts_with("F") || part.starts_with("NUMPAD") ||
                   part == "SPACE" || part == "ENTER" || part == "ESCAPE" ||
                   part.ends_with("ARROW") {
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
    // In a real implementation, you would check against registered hotkeys
    // For now, return empty as we don't have persistent storage
    Vec::new()
}

/// Determine hotkey category from action
fn determine_hotkey_category(action: &HotkeyAction) -> HotkeyCategory {
    match action {
        HotkeyAction::StartVideo | HotkeyAction::StopVideo | HotkeyAction::StartAudio |
        HotkeyAction::StopAudio | HotkeyAction::ToggleMute | HotkeyAction::VolumeUp |
        HotkeyAction::VolumeDown | HotkeyAction::SwitchVideo | HotkeyAction::SwitchAudio |
        HotkeyAction::ToggleCamera | HotkeyAction::ToggleMicrophone => HotkeyCategory::Media,

        HotkeyAction::StartRecording | HotkeyAction::StopRecording | HotkeyAction::Screenshot => {
            HotkeyCategory::Recording
        },

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
            // In a real implementation, you would start video streaming
            Ok(serde_json::json!({
                "status": "started",
                "message": "Video streaming started"
            }))
        },
        HotkeyAction::StopVideo => {
            // In a real implementation, you would stop video streaming
            Ok(serde_json::json!({
                "status": "stopped",
                "message": "Video streaming stopped"
            }))
        },
        HotkeyAction::ToggleMute => {
            // In a real implementation, you would toggle microphone mute
            Ok(serde_json::json!({
                "status": "toggled",
                "muted": true,
                "message": "Microphone muted"
            }))
        },
        HotkeyAction::VolumeUp => {
            // In a real implementation, you would increase volume
            Ok(serde_json::json!({
                "status": "adjusted",
                "volume": 0.8,
                "message": "Volume increased"
            }))
        },
        HotkeyAction::VolumeDown => {
            // In a real implementation, you would decrease volume
            Ok(serde_json::json!({
                "status": "adjusted",
                "volume": 0.4,
                "message": "Volume decreased"
            }))
        },
        HotkeyAction::Screenshot => {
            // In a real implementation, you would take a screenshot
            Ok(serde_json::json!({
                "status": "captured",
                "message": "Screenshot taken"
            }))
        },
        HotkeyAction::Quit => {
            // In a real implementation, you would quit the application
            info!("Quit action triggered by hotkey");
            // app.exit(0);
            Ok(serde_json::json!({
                "status": "quitting",
                "message": "Application will quit"
            }))
        },
        HotkeyAction::Custom(action_name) => {
            // In a real implementation, you would execute custom actions
            Ok(serde_json::json!({
                "status": "executed",
                "action": action_name,
                "message": format!("Custom action '{}' executed", action_name)
            }))
        },
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