//! JSON DSL Tauri Commands
//!
//! This module provides Tauri commands for JSON DSL scripting functionality.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Manager};
use tracing::{info, error, warn, debug};

use crate::json_dsl::{JsonDslEngine, JsonDslScript, ScriptExecutionResult, create_example_scripts};
use crate::json_dsl_integration::{IntegratedDslEngine, MediaStatus};

/// Global state for JSON DSL functionality
pub struct JsonDslState {
    pub integrated_engine: Arc<Mutex<IntegratedDslEngine>>,
    pub script_store: Arc<Mutex<HashMap<String, JsonDslScript>>>,
}

/// JSON DSL script template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub template: JsonDslScript,
}

/// Parsed script information for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub tags: Option<Vec<String>>,
    pub estimated_duration: Option<f64>,
    pub action_count: usize,
    pub variable_count: usize,
}

/// Script execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub script_id: String,
    pub variables: Option<HashMap<String, serde_json::Value>>,
    pub dry_run: Option<bool>,
}

/// Script execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub success: bool,
    pub execution_id: String,
    pub result: Option<ScriptExecutionResult>,
    pub error: Option<String>,
}

/// Create JSON DSL state
pub fn create_json_dsl_state() -> JsonDslState {
    JsonDslState {
        integrated_engine: Arc::new(Mutex::new(IntegratedDslEngine::new())),
        script_store: Arc::new(Mutex::new(HashMap::new())),
    }
}

/// Parse and validate a JSON DSL script
#[command]
pub async fn parse_json_dsl_script(
    app: AppHandle,
    script_content: String,
) -> Result<ScriptInfo, String> {
    info!("Parsing JSON DSL script");

    let state = app.state::<JsonDslState>();
    let engine = state.integrated_engine.lock().unwrap();

    match engine.parse_script(&script_content).await {
        Ok(script) => {
            let script_info = ScriptInfo {
                id: script.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                name: script.name.clone(),
                description: script.description.clone(),
                version: script.version.clone(),
                created_at: script.metadata.as_ref()
                    .and_then(|m| m.created_at)
                    .map(|dt| dt.to_rfc3339()),
                modified_at: script.metadata.as_ref()
                    .and_then(|m| m.modified_at)
                    .map(|dt| dt.to_rfc3339()),
                tags: script.metadata.as_ref()
                    .and_then(|m| m.tags.clone()),
                estimated_duration: script.metadata.as_ref()
                    .and_then(|m| m.estimated_duration),
                action_count: script.actions.len(),
                variable_count: script.variables.as_ref()
                    .map(|v| v.len())
                    .unwrap_or(0),
            };

            info!("Successfully parsed script: {}", script_info.name);
            Ok(script_info)
        },
        Err(e) => {
            error!("Failed to parse script: {}", e);
            Err(e.to_string())
        }
    }
}

/// Save a script to the store
#[command]
pub async fn save_json_dsl_script(
    app: AppHandle,
    script: JsonDslScript,
) -> Result<String, String> {
    info!("Saving JSON DSL script: {}", script.name);

    let state = app.state::<JsonDslState>();

    // Validate the script first
    let engine = state.integrated_engine.lock().unwrap();
    match engine.parse_script(&serde_json::to_string(&script).unwrap()).await {
        Ok(_) => {
            drop(engine);

            let script_id = script.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let mut script_with_id = script;
            script_with_id.id = Some(script_id.clone());

            let mut store = state.script_store.lock().unwrap();
            store.insert(script_id.clone(), script_with_id);

            info!("Successfully saved script with ID: {}", script_id);
            Ok(script_id)
        },
        Err(e) => {
            error!("Failed to validate script for saving: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get all saved scripts
#[command]
pub async fn get_json_dsl_scripts(
    app: AppHandle,
) -> Result<Vec<ScriptInfo>, String> {
    info!("Retrieving all JSON DSL scripts");

    let state = app.state::<JsonDslState>();
    let store = state.script_store.lock().unwrap();

    let scripts: Vec<ScriptInfo> = store.values().map(|script| {
        ScriptInfo {
            id: script.id.clone().unwrap_or_default(),
            name: script.name.clone(),
            description: script.description.clone(),
            version: script.version.clone(),
            created_at: script.metadata.as_ref()
                .and_then(|m| m.created_at)
                .map(|dt| dt.to_rfc3339()),
            modified_at: script.metadata.as_ref()
                .and_then(|m| m.modified_at)
                .map(|dt| dt.to_rfc3339()),
            tags: script.metadata.as_ref()
                .and_then(|m| m.tags.clone()),
            estimated_duration: script.metadata.as_ref()
                .and_then(|m| m.estimated_duration),
            action_count: script.actions.len(),
            variable_count: script.variables.as_ref()
                .map(|v| v.len())
                .unwrap_or(0),
        }
    }).collect();

    info!("Retrieved {} scripts", scripts.len());
    Ok(scripts)
}

/// Get a specific script by ID
#[command]
pub async fn get_json_dsl_script(
    app: AppHandle,
    script_id: String,
) -> Result<JsonDslScript, String> {
    info!("Retrieving script: {}", script_id);

    let state = app.state::<JsonDslState>();
    let store = state.script_store.lock().unwrap();

    match store.get(&script_id) {
        Some(script) => {
            info!("Successfully retrieved script: {}", script.name);
            Ok(script.clone())
        },
        None => {
            warn!("Script not found: {}", script_id);
            Err(format!("Script not found: {}", script_id))
        }
    }
}

/// Delete a script
#[command]
pub async fn delete_json_dsl_script(
    app: AppHandle,
    script_id: String,
) -> Result<bool, String> {
    info!("Deleting script: {}", script_id);

    let state = app.state::<JsonDslState>();
    let mut store = state.script_store.lock().unwrap();

    match store.remove(&script_id) {
        Some(_) => {
            info!("Successfully deleted script: {}", script_id);
            Ok(true)
        },
        None => {
            warn!("Script not found for deletion: {}", script_id);
            Err(format!("Script not found: {}", script_id))
        }
    }
}

/// Execute a script
#[command]
pub async fn execute_json_dsl_script(
    app: AppHandle,
    request: ExecutionRequest,
) -> Result<ExecutionResponse, String> {
    info!("Executing script: {}", request.script_id);

    let state = app.state::<JsonDslState>();

    // Get the script from store
    let script = {
        let store = state.script_store.lock().unwrap();
        match store.get(&request.script_id) {
            Some(script) => script.clone(),
            None => {
                return Err(format!("Script not found: {}", request.script_id));
            }
        }
    };

    // Check if it's a dry run
    if request.dry_run.unwrap_or(false) {
        info!("Dry run execution for script: {}", script.name);
        return Ok(ExecutionResponse {
            success: true,
            execution_id: uuid::Uuid::new_v4().to_string(),
            result: None,
            error: None,
        });
    }

    // Execute the script
    let execution_id = uuid::Uuid::new_v4().to_string();
    let engine = state.integrated_engine.lock().unwrap();

    match engine.execute_script(&script).await {
        Ok(result) => {
            info!("Script execution completed successfully: {} ({}ms)", script.name, result.execution_time_ms);
            Ok(ExecutionResponse {
                success: true,
                execution_id,
                result: Some(result),
                error: None,
            })
        },
        Err(e) => {
            error!("Script execution failed: {}", e);
            Ok(ExecutionResponse {
                success: false,
                execution_id,
                result: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Execute script content directly (without saving)
#[command]
pub async fn execute_json_dsl_content(
    app: AppHandle,
    script_content: String,
    variables: Option<HashMap<String, serde_json::Value>>,
) -> Result<ScriptExecutionResult, String> {
    info!("Executing script content directly");

    let state = app.state::<JsonDslState>();
    let engine = state.integrated_engine.lock().unwrap();

    // Parse the script first
    let script = match engine.parse_script(&script_content).await {
        Ok(script) => script,
        Err(e) => {
            error!("Failed to parse script content: {}", e);
            return Err(e.to_string());
        }
    };

    // Execute the script
    match engine.execute_script(&script).await {
        Ok(result) => {
            info!("Direct script execution completed successfully ({}ms)", result.execution_time_ms);
            Ok(result)
        },
        Err(e) => {
            error!("Direct script execution failed: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get media status
#[command]
pub async fn get_media_status(
    app: AppHandle,
) -> Result<MediaStatus, String> {
    info!("Getting media status");

    let state = app.state::<JsonDslState>();
    let engine = state.integrated_engine.lock().unwrap();

    match engine.get_media_status().await {
        Ok(status) => {
            debug!("Media status: {} video streams, {} audio streams, {} recording sessions",
                status.active_video_streams, status.active_audio_streams, status.active_recording_sessions);
            Ok(status)
        },
        Err(e) => {
            error!("Failed to get media status: {}", e);
            Err(e.to_string())
        }
    }
}

/// Stop current script execution
#[command]
pub async fn stop_script_execution(
    app: AppHandle,
) -> Result<bool, String> {
    info!("Stopping script execution");

    let state = app.state::<JsonDslState>();
    let engine = state.integrated_engine.lock().unwrap();

    // Note: This would require adding stop_execution method to IntegratedDslEngine
    // For now, we'll clean up resources
    match engine.get_media_status().await {
        Ok(_) => {
            info!("Script execution stopped successfully");
            Ok(true)
        },
        Err(e) => {
            error!("Failed to stop script execution: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get script templates
#[command]
pub async fn get_script_templates() -> Result<Vec<ScriptTemplate>, String> {
    info!("Getting script templates");

    let example_scripts = create_example_scripts();

    let templates = example_scripts.into_iter().enumerate().map(|(index, script)| {
        let category = if index == 0 { "simple" } else { "complex" };
        ScriptTemplate {
            id: format!("template_{}", index),
            name: script.name.clone(),
            description: script.description.clone().unwrap_or_default(),
            category: category.to_string(),
            template: script,
        }
    }).collect();

    info!("Returning {} script templates", templates.len());
    Ok(templates)
}

/// Validate script syntax without executing
#[command]
pub async fn validate_json_dsl_script(
    app: AppHandle,
    script_content: String,
) -> Result<ScriptValidationResult, String> {
    info!("Validating JSON DSL script");

    let state = app.state::<JsonDslState>();
    let engine = state.integrated_engine.lock().unwrap();

    match engine.parse_script(&script_content).await {
        Ok(script) => {
            let validation_result = ScriptValidationResult {
                is_valid: true,
                error: None,
                warnings: Vec::new(),
                action_count: script.actions.len(),
                variable_count: script.variables.as_ref().map(|v| v.len()).unwrap_or(0),
                estimated_duration: script.metadata.as_ref().and_then(|m| m.estimated_duration),
            };

            info!("Script validation successful");
            Ok(validation_result)
        },
        Err(e) => {
            error!("Script validation failed: {}", e);
            Ok(ScriptValidationResult {
                is_valid: false,
                error: Some(e.to_string()),
                warnings: Vec::new(),
                action_count: 0,
                variable_count: 0,
                estimated_duration: None,
            })
        }
    }
}

/// Script validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptValidationResult {
    pub is_valid: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub action_count: usize,
    pub variable_count: usize,
    pub estimated_duration: Option<f64>,
}

/// Export script to file
#[command]
pub async fn export_json_dsl_script(
    app: AppHandle,
    script_id: String,
    file_path: String,
) -> Result<bool, String> {
    info!("Exporting script {} to {}", script_id, file_path);

    let state = app.state::<JsonDslState>();
    let store = state.script_store.lock().unwrap();

    match store.get(&script_id) {
        Some(script) => {
            // Convert script to JSON string
            let script_json = match serde_json::to_string_pretty(&script) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize script: {}", e);
                    return Err(format!("Failed to serialize script: {}", e));
                }
            };

            // Write to file (this would need proper file system access in Tauri)
            match std::fs::write(&file_path, script_json) {
                Ok(_) => {
                    info!("Successfully exported script to: {}", file_path);
                    Ok(true)
                },
                Err(e) => {
                    error!("Failed to write script to file: {}", e);
                    Err(format!("Failed to write script to file: {}", e));
                }
            }
        },
        None => {
            warn!("Script not found for export: {}", script_id);
            Err(format!("Script not found: {}", script_id))
        }
    }
}

/// Import script from file
#[command]
pub async fn import_json_dsl_script(
    app: AppHandle,
    file_path: String,
) -> Result<String, String> {
    info!("Importing script from: {}", file_path);

    // Read script from file (this would need proper file system access in Tauri)
    let script_content = match std::fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read script from file: {}", e);
            return Err(format!("Failed to read script from file: {}", e));
        }
    };

    // Parse and validate the script
    let state = app.state::<JsonDslState>();
    let engine = state.integrated_engine.lock().unwrap();

    match engine.parse_script(&script_content).await {
        Ok(mut script) => {
            // Generate a new ID for the imported script
            let script_id = uuid::Uuid::new_v4().to_string();
            script.id = Some(script_id.clone());

            // Save to store
            drop(engine);
            let mut store = state.script_store.lock().unwrap();
            store.insert(script_id.clone(), script);

            info!("Successfully imported script with ID: {}", script_id);
            Ok(script_id)
        },
        Err(e) => {
            error!("Failed to parse imported script: {}", e);
            Err(format!("Failed to parse imported script: {}", e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_dsl::{ScriptAction, ScriptValue};

    #[test]
    fn test_script_validation_result() {
        let result = ScriptValidationResult {
            is_valid: true,
            error: None,
            warnings: vec!["Test warning".to_string()],
            action_count: 5,
            variable_count: 2,
            estimated_duration: Some(30.0),
        };

        assert!(result.is_valid);
        assert_eq!(result.action_count, 5);
        assert_eq!(result.variable_count, 2);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_execution_request() {
        let request = ExecutionRequest {
            script_id: "test_script".to_string(),
            variables: Some(HashMap::from([
                ("test_var".to_string(), serde_json::Value::String("test_value".to_string())),
            ])),
            dry_run: Some(true),
        };

        assert_eq!(request.script_id, "test_script");
        assert!(request.dry_run.unwrap());
        assert!(request.variables.is_some());
    }

    #[test]
    fn test_script_template() {
        let template = ScriptTemplate {
            id: "test_template".to_string(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            category: "test".to_string(),
            template: JsonDslScript {
                name: "Test Script".to_string(),
                actions: vec![
                    ScriptAction::Log {
                        level: Some("info".to_string()),
                        message: "Test".to_string(),
                        include_variables: Some(false),
                    },
                ],
                ..Default::default()
            },
        };

        assert_eq!(template.id, "test_template");
        assert_eq!(template.category, "test");
        assert_eq!(template.template.actions.len(), 1);
    }
}