//! Scripting Commands
//!
//! Tauri commands for script execution, management, and automation using Rhai scripting engine.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{command, AppHandle, State};
use tracing::{debug, error, info, warn};

use crate::AppState;

/// Script execution result
#[derive(Debug, Serialize, Clone)]
pub struct ScriptExecutionResult {
    pub success: bool,
    pub script_id: String,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub variables: HashMap<String, serde_json::Value>,
    pub logs: Vec<String>,
}

/// Script definition
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub language: ScriptLanguage,
    pub author: Option<String>,
    pub version: String,
    pub tags: Vec<String>,
    pub parameters: Vec<ScriptParameter>,
    pub enabled: bool,
    pub auto_start: bool,
    pub created_at: String,
    pub modified_at: String,
}

/// Script language
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScriptLanguage {
    Rhai,
    JavaScript,
    Python,
    Custom(String),
}

/// Script parameter definition
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub parameter_type: ScriptParameterType,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub options: Option<Vec<String>>,
}

/// Script parameter type
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScriptParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    File,
    Directory,
}

/// Script execution request
#[derive(Debug, Deserialize)]
pub struct ExecuteScriptRequest {
    pub script_id: String,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub timeout_ms: Option<u64>,
    pub async_execution: bool,
}

/// Script creation request
#[derive(Debug, Deserialize)]
pub struct CreateScriptRequest {
    pub name: String,
    pub description: String,
    pub content: String,
    pub language: ScriptLanguage,
    pub parameters: Option<Vec<ScriptParameter>>,
    pub tags: Option<Vec<String>>,
}

/// Script update request
#[derive(Debug, Deserialize)]
pub struct UpdateScriptRequest {
    pub script_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub parameters: Option<Vec<ScriptParameter>>,
    pub enabled: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// Script list response
#[derive(Debug, Serialize)]
pub struct ScriptListResponse {
    pub success: bool,
    pub message: String,
    pub scripts: Vec<ScriptDefinition>,
}

/// Script validation result
#[derive(Debug, Serialize)]
pub struct ScriptValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub syntax_highlight: Option<Vec<SyntaxToken>>,
}

/// Syntax token for code highlighting
#[derive(Debug, Serialize)]
pub struct SyntaxToken {
    pub text: String,
    pub token_type: String,
    pub start_position: usize,
    pub end_position: usize,
}

/// Script template
#[derive(Debug, Serialize)]
pub struct ScriptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub content: String,
    pub parameters: Vec<ScriptParameter>,
}

/// Shared scripting state
pub struct ScriptingState {
    pub scripts: HashMap<String, ScriptDefinition>,
    pub execution_history: Vec<ScriptExecutionResult>,
    pub templates: HashMap<String, ScriptTemplate>,
    pub rhai_engine: Arc<StdMutex<rhai::Engine>>,
}

impl Default for ScriptingState {
    fn default() -> Self {
        let mut engine = rhai::Engine::new();

        // Register custom functions and types
        register_scripting_functions(&mut engine);

        Self {
            scripts: HashMap::new(),
            execution_history: Vec::new(),
            templates: HashMap::new(),
            rhai_engine: Arc::new(StdMutex::new(engine)),
        }
    }
}

/// Execute a script
#[command]
pub async fn execute_script(
    request: ExecuteScriptRequest,
    app: AppHandle,
) -> Result<ScriptExecutionResult, String> {
    info!("Executing script: {}", request.script_id);

    let scripting_state = app.state::<std::sync::Mutex<ScriptingState>>();
    let state = scripting_state
        .lock()
        .map_err(|e| format!("Failed to lock scripting state: {}", e))?;

    let script = match state.scripts.get(&request.script_id) {
        Some(script) => script.clone(),
        None => {
            return Ok(ScriptExecutionResult {
                success: false,
                script_id: request.script_id,
                output: String::new(),
                error: Some(format!("Script with ID '{}' not found", request.script_id)),
                execution_time_ms: 0,
                variables: HashMap::new(),
                logs: vec![],
            });
        }
    };

    if !script.enabled {
        return Ok(ScriptExecutionResult {
            success: false,
            script_id: request.script_id,
            output: String::new(),
            error: Some("Script is disabled".to_string()),
            execution_time_ms: 0,
            variables: HashMap::new(),
            logs: vec![],
        });
    }

    let start_time = std::time::Instant::now();

    match script.language {
        ScriptLanguage::Rhai => execute_rhai_script(&script, request.parameters, &state),
        ScriptLanguage::JavaScript => Ok(ScriptExecutionResult {
            success: false,
            script_id: request.script_id,
            output: String::new(),
            error: Some("JavaScript execution not implemented".to_string()),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            variables: HashMap::new(),
            logs: vec![],
        }),
        ScriptLanguage::Python => Ok(ScriptExecutionResult {
            success: false,
            script_id: request.script_id,
            output: String::new(),
            error: Some("Python execution not implemented".to_string()),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            variables: HashMap::new(),
            logs: vec![],
        }),
        ScriptLanguage::Custom(_) => Ok(ScriptExecutionResult {
            success: false,
            script_id: request.script_id,
            output: String::new(),
            error: Some("Custom language execution not implemented".to_string()),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            variables: HashMap::new(),
            logs: vec![],
        }),
    }
}

/// Create a new script
#[command]
pub async fn create_script(
    request: CreateScriptRequest,
    app: AppHandle,
) -> Result<ScriptDefinition, String> {
    info!("Creating new script: {}", request.name);

    // Generate unique ID
    let script_id = generate_script_id(&request.name);

    // Validate script syntax
    let validation_result = validate_script_syntax(&request.content, &request.language)?;
    if !validation_result.is_valid {
        return Err(format!(
            "Script syntax validation failed: {:?}",
            validation_result.errors
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let script = ScriptDefinition {
        id: script_id.clone(),
        name: request.name,
        description: request.description,
        content: request.content,
        language: request.language,
        author: None,
        version: "1.0.0".to_string(),
        tags: request.tags.unwrap_or_default(),
        parameters: request.parameters.unwrap_or_default(),
        enabled: true,
        auto_start: false,
        created_at: now.clone(),
        modified_at: now,
    };

    // Store script
    let scripting_state = app.state::<std::sync::Mutex<ScriptingState>>();
    let mut state = scripting_state
        .lock()
        .map_err(|e| format!("Failed to lock scripting state: {}", e))?;

    state.scripts.insert(script_id.clone(), script.clone());

    info!(
        "Script created successfully: {} ({})",
        script.name, script_id
    );
    Ok(script)
}

/// Update an existing script
#[command]
pub async fn update_script(
    request: UpdateScriptRequest,
    app: AppHandle,
) -> Result<ScriptDefinition, String> {
    info!("Updating script: {}", request.script_id);

    let scripting_state = app.state::<std::sync::Mutex<ScriptingState>>();
    let mut state = scripting_state
        .lock()
        .map_err(|e| format!("Failed to lock scripting state: {}", e))?;

    let mut script = match state.scripts.get(&request.script_id).cloned() {
        Some(script) => script,
        None => {
            return Err(format!("Script with ID '{}' not found", request.script_id));
        }
    };

    // Update fields
    if let Some(name) = request.name {
        script.name = name;
    }
    if let Some(description) = request.description {
        script.description = description;
    }
    if let Some(content) = request.content {
        // Validate syntax before updating
        let validation_result = validate_script_syntax(&content, &script.language)?;
        if !validation_result.is_valid {
            return Err(format!(
                "Script syntax validation failed: {:?}",
                validation_result.errors
            ));
        }
        script.content = content;
    }
    if let Some(parameters) = request.parameters {
        script.parameters = parameters;
    }
    if let Some(enabled) = request.enabled {
        script.enabled = enabled;
    }
    if let Some(tags) = request.tags {
        script.tags = tags;
    }

    script.modified_at = chrono::Utc::now().to_rfc3339();

    // Update script in storage
    state
        .scripts
        .insert(request.script_id.clone(), script.clone());

    info!("Script updated successfully: {}", script.name);
    Ok(script)
}

/// Delete a script
#[command]
pub async fn delete_script(script_id: String, app: AppHandle) -> Result<bool, String> {
    info!("Deleting script: {}", script_id);

    let scripting_state = app.state::<std::sync::Mutex<ScriptingState>>();
    let mut state = scripting_state
        .lock()
        .map_err(|e| format!("Failed to lock scripting state: {}", e))?;

    match state.scripts.remove(&script_id) {
        Some(script) => {
            info!("Script deleted successfully: {}", script.name);
            Ok(true)
        }
        None => {
            warn!("Script with ID '{}' not found for deletion", script_id);
            Ok(false)
        }
    }
}

/// Get all scripts
#[command]
pub async fn get_scripts(app: AppHandle) -> Result<ScriptListResponse, String> {
    info!("Getting all scripts");

    let scripting_state = app.state::<std::sync::Mutex<ScriptingState>>();
    let state = scripting_state
        .lock()
        .map_err(|e| format!("Failed to lock scripting state: {}", e))?;

    let scripts: Vec<ScriptDefinition> = state.scripts.values().cloned().collect();

    Ok(ScriptListResponse {
        success: true,
        message: format!("Retrieved {} scripts", scripts.len()),
        scripts,
    })
}

/// Get script by ID
#[command]
pub async fn get_script(
    script_id: String,
    app: AppHandle,
) -> Result<Option<ScriptDefinition>, String> {
    info!("Getting script: {}", script_id);

    let scripting_state = app.state::<std::sync::Mutex<ScriptingState>>();
    let state = scripting_state
        .lock()
        .map_err(|e| format!("Failed to lock scripting state: {}", e))?;

    Ok(state.scripts.get(&script_id).cloned())
}

/// Validate script syntax
#[command]
pub async fn validate_script(
    content: String,
    language: ScriptLanguage,
) -> Result<ScriptValidationResult, String> {
    info!("Validating script syntax");

    validate_script_syntax(&content, &language)
        .map_err(|e| format!("Script validation failed: {}", e))
}

/// Get script templates
#[command]
pub async fn get_script_templates() -> Result<Vec<ScriptTemplate>, String> {
    info!("Getting script templates");

    let templates = vec![
        ScriptTemplate {
            id: "start_video_streaming".to_string(),
            name: "Start Video Streaming".to_string(),
            description: "Start video streaming from a file".to_string(),
            category: "Video Control".to_string(),
            content: r#"
// Start video streaming
let video_path = parameters.get("video_path") ?? "default.mp4";

// Call the virtual device function
virtual_devices::start_webcam_streaming(video_path);

print("Video streaming started: " + video_path);"#
                .to_string(),
            parameters: vec![ScriptParameter {
                name: "video_path".to_string(),
                description: "Path to video file".to_string(),
                parameter_type: ScriptParameterType::File,
                default_value: None,
                required: true,
                min_value: None,
                max_value: None,
                options: None,
            }],
        },
        ScriptTemplate {
            id: "start_recording".to_string(),
            name: "Start Recording".to_string(),
            description: "Start recording video and audio".to_string(),
            category: "Recording".to_string(),
            content: r#"
// Start recording
let output_path = parameters.get("output_path") ?? "recording.mp4";
let duration = parameters.get("duration") ?? 60;

// Configure recording
let config = {
    "video_resolution": "1080p",
    "video_quality": "high",
    "audio_bitrate": 192000
};

// Start recording
recording::start_recording(output_path, config);

print("Recording started: " + output_path);
print("Duration: " + duration + " seconds");"#
                .to_string(),
            parameters: vec![
                ScriptParameter {
                    name: "output_path".to_string(),
                    description: "Output file path".to_string(),
                    parameter_type: ScriptParameterType::File,
                    default_value: None,
                    required: true,
                    min_value: None,
                    max_value: None,
                    options: None,
                },
                ScriptParameter {
                    name: "duration".to_string(),
                    description: "Recording duration in seconds".to_string(),
                    parameter_type: ScriptParameterType::Number,
                    default_value: Some(serde_json::json!(60)),
                    required: false,
                    min_value: Some(1.0),
                    max_value: Some(3600.0),
                    options: None,
                },
            ],
        },
        ScriptTemplate {
            id: "device_control".to_string(),
            name: "Device Control".to_string(),
            description: "Control virtual devices".to_string(),
            category: "Device Control".to_string(),
            content: r#"
// Device control script
let action = parameters.get("action") ?? "status";

match action {
    "status" => {
        // Get device status
        let status = virtual_devices::get_virtual_device_status();
        print("Webcam active: " + status.webcam_active);
        print("Microphone active: " + status.microphone_active);
    },
    "start_all" => {
        // Start all virtual devices
        virtual_devices::initialize_webcam("DirectShow");
        virtual_devices::initialize_microphone("WASAPI");
        print("All virtual devices started");
    },
    "stop_all" => {
        // Stop all virtual devices
        virtual_devices::stop_webcam_streaming();
        virtual_devices::stop_microphone_streaming();
        print("All virtual devices stopped");
    },
    _ => {
        print("Unknown action: " + action);
    }
}"#
            .to_string(),
            parameters: vec![ScriptParameter {
                name: "action".to_string(),
                description: "Action to perform".to_string(),
                parameter_type: ScriptParameterType::String,
                default_value: Some(serde_json::json!("status")),
                required: false,
                min_value: None,
                max_value: None,
                options: Some(vec![
                    "status".to_string(),
                    "start_all".to_string(),
                    "stop_all".to_string(),
                ]),
            }],
        },
    ];

    Ok(templates)
}

/// Generate unique script ID
fn generate_script_id(name: &str) -> String {
    let sanitized_name = name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    let timestamp = chrono::Utc::now().timestamp_millis();
    format!("{}_{}", sanitized_name, timestamp)
}

/// Validate script syntax
fn validate_script_syntax(
    content: &str,
    language: &ScriptLanguage,
) -> Result<ScriptValidationResult> {
    match language {
        ScriptLanguage::Rhai => {
            let engine = rhai::Engine::new();

            match engine.compile(content) {
                Ok(_) => Ok(ScriptValidationResult {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    syntax_highlight: None,
                }),
                Err(e) => Ok(ScriptValidationResult {
                    is_valid: false,
                    errors: vec![format!("Syntax error: {}", e)],
                    warnings: vec![],
                    syntax_highlight: None,
                }),
            }
        }
        _ => Ok(ScriptValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec!["Syntax validation not implemented for this language".to_string()],
            syntax_highlight: None,
        }),
    }
}

/// Execute Rhai script
fn execute_rhai_script(
    script: &ScriptDefinition,
    parameters: Option<HashMap<String, serde_json::Value>>,
    state: &ScriptingState,
) -> ScriptExecutionResult {
    let start_time = std::time::Instant::now();

    let engine = state.rhai_engine.lock().unwrap();
    let mut scope = rhai::Scope::new();

    // Add parameters to scope
    if let Some(params) = parameters {
        for (key, value) in params {
            scope.push(key, rhai::Dynamic::from(value.to_string()));
        }
    }

    // Add built-in functions and variables
    scope.push(
        "print",
        rhai::Dynamic::from_fn(move |args: rhai::NativeCallArgs| {
            let message = args
                .iter()
                .map(|arg| arg.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            info!("Script output: {}", message);
            rhai::Dynamic::from(())
        }),
    );

    match engine.eval_with_scope::<rhai::Dynamic>(&mut scope, &script.content) {
        Ok(result) => {
            let execution_time = start_time.elapsed().as_millis() as u64;

            // Extract variables from scope
            let mut variables = HashMap::new();
            for (name, value) in scope.iter() {
                variables.insert(
                    name.clone(),
                    serde_json::to_value(&value).unwrap_or_default(),
                );
            }

            ScriptExecutionResult {
                success: true,
                script_id: script.id.clone(),
                output: result.to_string(),
                error: None,
                execution_time_ms: execution_time,
                variables,
                logs: vec![],
            }
        }
        Err(e) => {
            let execution_time = start_time.elapsed().as_millis() as u64;

            ScriptExecutionResult {
                success: false,
                script_id: script.id.clone(),
                output: String::new(),
                error: Some(format!("Execution error: {}", e)),
                execution_time_ms: execution_time,
                variables: HashMap::new(),
                logs: vec![],
            }
        }
    }
}

/// Register scripting functions
fn register_scripting_functions(engine: &mut rhai::Engine) {
    // Register virtual device functions
    engine.register_fn("start_webcam_streaming", |path: &str| {
        info!("Script: Starting webcam streaming: {}", path);
        // In a real implementation, this would call the actual function
    });

    engine.register_fn("stop_webcam_streaming", || {
        info!("Script: Stopping webcam streaming");
        // In a real implementation, this would call the actual function
    });

    engine.register_fn("start_microphone_streaming", |path: &str| {
        info!("Script: Starting microphone streaming: {}", path);
        // In a real implementation, this would call the actual function
    });

    engine.register_fn("stop_microphone_streaming", || {
        info!("Script: Stopping microphone streaming");
        // In a real implementation, this would call the actual function
    });

    // Register recording functions
    engine.register_fn("start_recording", |path: &str, config: rhai::Map| {
        info!("Script: Starting recording: {}", path);
        info!("Recording config: {:?}", config);
        // In a real implementation, this would call the actual function
    });

    engine.register_fn("stop_recording", || {
        info!("Script: Stopping recording");
        // In a real implementation, this would call the actual function
    });

    // Register utility functions
    engine.register_fn("sleep", |seconds: i64| {
        std::thread::sleep(std::time::Duration::from_secs(seconds as u64));
    });

    engine.register_fn("timestamp", || chrono::Utc::now().timestamp());
}

/// Initialize scripting system
pub fn init_scripting_system() -> ScriptingState {
    info!("Initializing scripting system");
    let mut state = ScriptingState::default();

    // Load built-in templates
    let templates = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(get_script_templates())
        .unwrap_or_default();

    for template in templates {
        state.templates.insert(template.id.clone(), template);
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_id_generation() {
        let id1 = generate_script_id("Test Script");
        let id2 = generate_script_id("Test Script");

        assert_ne!(id1, id2); // Should be unique due to timestamp
        assert!(id1.starts_with("test_script"));
        assert!(id2.starts_with("test_script"));
    }

    #[test]
    fn test_rhai_script_validation() {
        let valid_script = "print(\"Hello, World!\");";
        let invalid_script = "print(\"Hello, World!\""; // Missing closing parenthesis

        let valid_result = validate_script_syntax(valid_script, &ScriptLanguage::Rhai).unwrap();
        assert!(valid_result.is_valid);

        let invalid_result = validate_script_syntax(invalid_script, &ScriptLanguage::Rhai).unwrap();
        assert!(!invalid_result.is_valid);
        assert!(!invalid_result.errors.is_empty());
    }

    #[test]
    fn test_script_parameter_types() {
        let param = ScriptParameter {
            name: "test_param".to_string(),
            description: "Test parameter".to_string(),
            parameter_type: ScriptParameterType::Number,
            default_value: Some(serde_json::json!(42.0)),
            required: false,
            min_value: Some(0.0),
            max_value: Some(100.0),
            options: None,
        };

        assert_eq!(param.name, "test_param");
        assert!(matches!(param.parameter_type, ScriptParameterType::Number));
        assert_eq!(param.default_value, Some(serde_json::json!(42.0)));
    }

    #[test]
    fn test_scripting_state_default() {
        let state = ScriptingState::default();
        assert!(state.scripts.is_empty());
        assert!(state.execution_history.is_empty());
        assert!(!state.templates.is_empty()); // Should have built-in templates
    }
}
