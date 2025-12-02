//! JSON DSL Scripting Engine
//!
//! This module provides a JSON-based DSL for executing user-defined sequences
//! with support for video playback, audio control, waiting, and conditional logic.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// JSON DSL Script definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDslScript {
    /// Unique identifier for the script
    pub id: Option<String>,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Script version
    pub version: Option<String>,
    /// Script metadata
    pub metadata: Option<ScriptMetadata>,
    /// List of actions to execute
    pub actions: Vec<ScriptAction>,
    /// Script variables
    pub variables: Option<HashMap<String, ScriptValue>>,
    /// Script configuration
    pub config: Option<ScriptConfig>,
}

/// Script metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadata {
    /// Author of the script
    pub author: Option<String>,
    /// Creation date
    pub created_at: Option<DateTime<Utc>>,
    /// Last modified date
    pub modified_at: Option<DateTime<Utc>>,
    /// Tags for categorization
    pub tags: Option<Vec<String>>,
    /// Estimated execution time
    pub estimated_duration: Option<f64>,
}

/// Script configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Whether to stop on error
    pub stop_on_error: Option<bool>,
    /// Default timeout for actions (seconds)
    pub default_timeout: Option<f64>,
    /// Whether to loop the script
    pub loop_script: Option<bool>,
    /// Number of times to loop (None = infinite)
    pub loop_count: Option<u32>,
    /// Delay between loops (seconds)
    pub loop_delay: Option<f64>,
}

/// Script action - the core building block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScriptAction {
    /// Video playback actions
    PlayVideo {
        /// Path to video file
        path: String,
        /// Start time (seconds, optional)
        start_time: Option<f64>,
        /// Duration to play (seconds, optional)
        duration: Option<f64>,
        /// Whether to loop
        loop_video: Option<bool>,
        /// Target device (webcam, virtual, etc.)
        device: Option<String>,
        /// Volume (0.0-1.0)
        volume: Option<f64>,
    },

    /// Audio playback actions
    PlayAudio {
        /// Path to audio file
        path: String,
        /// Start time (seconds)
        start_time: Option<f64>,
        /// Duration to play (seconds)
        duration: Option<f64>,
        /// Whether to loop
        loop_audio: Option<bool>,
        /// Target device
        device: Option<String>,
        /// Volume (0.0-1.0)
        volume: Option<f64>,
    },

    /// Stop media playback
    StopMedia {
        /// Type of media to stop (video, audio, all)
        media_type: Option<String>,
        /// Specific device to stop
        device: Option<String>,
    },

    /// Wait/delay actions
    Wait {
        /// Duration in seconds
        duration: f64,
        /// Optional description for logging
        description: Option<String>,
    },

    /// Recording actions
    StartRecording {
        /// Output file path
        output_path: String,
        /// Recording quality
        quality: Option<String>,
        /// Whether to record video
        record_video: Option<bool>,
        /// Whether to record audio
        record_audio: Option<bool>,
        /// Maximum duration (seconds)
        max_duration: Option<f64>,
    },

    StopRecording {
        /// Optional save path override
        save_path: Option<String>,
    },

    /// Virtual device control
    SetVirtualDevice {
        /// Device type (webcam, microphone)
        device_type: String,
        /// Action (start, stop, restart)
        action: String,
        /// Source file or input
        source: Option<String>,
    },

    /// System actions
    ExecuteCommand {
        /// Command to execute
        command: String,
        /// Arguments
        args: Option<Vec<String>>,
        /// Working directory
        working_dir: Option<String>,
        /// Timeout in seconds
        timeout: Option<f64>,
    },

    /// Variable operations
    SetVariable {
        /// Variable name
        name: String,
        /// Variable value
        value: ScriptValue,
    },

    /// Conditional logic
    If {
        /// Condition to evaluate
        condition: ScriptCondition,
        /// Actions to execute if condition is true
        then_actions: Vec<ScriptAction>,
        /// Optional actions to execute if condition is false
        else_actions: Option<Vec<ScriptAction>>,
    },

    /// Loop actions
    While {
        /// Loop condition
        condition: ScriptCondition,
        /// Actions to execute in loop
        actions: Vec<ScriptAction>,
        /// Maximum iterations (optional)
        max_iterations: Option<u32>,
    },

    For {
        /// Variable name for loop counter
        variable: String,
        /// Start value
        from: i32,
        /// End value (exclusive)
        to: i32,
        /// Step increment
        step: Option<i32>,
        /// Actions to execute
        actions: Vec<ScriptAction>,
    },

    /// Log messages
    Log {
        /// Log level (info, warn, error, debug)
        level: Option<String>,
        /// Message to log
        message: String,
        /// Include variables in message
        include_variables: Option<bool>,
    },

    /// Custom function calls
    CallFunction {
        /// Function name
        function: String,
        /// Parameters
        parameters: Option<HashMap<String, ScriptValue>>,
    },
}

/// Script value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScriptValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<ScriptValue>),
    Object(HashMap<String, ScriptValue>),
    Null,
}

/// Script condition for conditional logic
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operator")]
pub enum ScriptCondition {
    /// Equality comparison
    Equals {
        left: ScriptValue,
        right: ScriptValue,
    },
    /// Not equal comparison
    NotEquals {
        left: ScriptValue,
        right: ScriptValue,
    },
    /// Greater than comparison
    GreaterThan {
        left: ScriptValue,
        right: ScriptValue,
    },
    /// Less than comparison
    LessThan {
        left: ScriptValue,
        right: ScriptValue,
    },
    /// Greater than or equal
    GreaterThanOrEqual {
        left: ScriptValue,
        right: ScriptValue,
    },
    /// Less than or equal
    LessThanOrEqual {
        left: ScriptValue,
        right: ScriptValue,
    },
    /// Logical AND
    And { conditions: Vec<ScriptCondition> },
    /// Logical OR
    Or { conditions: Vec<ScriptCondition> },
    /// Logical NOT
    Not { condition: Box<ScriptCondition> },
    /// Check if variable exists
    VariableExists { name: String },
    /// Check if file exists
    FileExists { path: String },
}

/// Script execution context
#[derive(Debug)]
pub struct ExecutionContext {
    /// Script variables
    variables: HashMap<String, ScriptValue>,
    /// Current execution state
    state: ExecutionState,
    /// Execution stack for nested actions
    call_stack: Vec<ExecutionContextFrame>,
    /// Start time
    start_time: DateTime<Utc>,
}

/// Execution state
#[derive(Debug, Clone)]
pub enum ExecutionState {
    Ready,
    Running,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

/// Execution context frame for nested calls
#[derive(Debug)]
struct ExecutionContextFrame {
    action_index: usize,
    loop_counter: u32,
    condition_result: Option<bool>,
}

/// Script execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Execution error message (if any)
    pub error: Option<String>,
    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of actions executed
    pub actions_executed: u32,
    /// Final variable values
    pub final_variables: HashMap<String, ScriptValue>,
    /// Execution log
    pub execution_log: Vec<String>,
}

/// JSON DSL Script Engine
#[derive(Debug)]
pub struct JsonDslEngine {
    /// Current execution context
    context: Option<ExecutionContext>,
    /// Available media devices
    media_devices: HashMap<String, MediaDevice>,
}

/// Media device information
#[derive(Debug, Clone)]
pub struct MediaDevice {
    pub device_id: String,
    pub device_type: MediaType,
    pub device_name: String,
    pub is_available: bool,
}

/// Media type enumeration
#[derive(Debug, Clone)]
pub enum MediaType {
    Video,
    Audio,
    Combined,
}

impl JsonDslEngine {
    /// Create a new JSON DSL engine
    pub fn new() -> Self {
        Self {
            context: None,
            media_devices: HashMap::new(),
        }
    }

    /// Parse JSON DSL script from string
    pub fn parse_script(&self, json_content: &str) -> Result<JsonDslScript> {
        let mut script: JsonDslScript = serde_json::from_str(json_content)
            .map_err(|e| anyhow!("Failed to parse JSON DSL script: {}", e))?;

        // Auto-generate ID if not provided
        if script.id.is_none() {
            script.id = Some(Uuid::new_v4().to_string());
        }

        // Validate script structure
        self.validate_script(&script)?;

        Ok(script)
    }

    /// Validate script structure and content
    fn validate_script(&self, script: &JsonDslScript) -> Result<()> {
        if script.name.is_empty() {
            return Err(anyhow!("Script name cannot be empty"));
        }

        if script.actions.is_empty() {
            return Err(anyhow!("Script must contain at least one action"));
        }

        // Validate each action
        for (index, action) in script.actions.iter().enumerate() {
            self.validate_action(action, index)?;
        }

        Ok(())
    }

    /// Validate individual action
    fn validate_action(&self, action: &ScriptAction, index: usize) -> Result<()> {
        match action {
            ScriptAction::PlayVideo { path, .. } => {
                if path.is_empty() {
                    return Err(anyhow!("Action {}: video path cannot be empty", index));
                }
            }
            ScriptAction::PlayAudio { path, .. } => {
                if path.is_empty() {
                    return Err(anyhow!("Action {}: audio path cannot be empty", index));
                }
            }
            ScriptAction::Wait { duration, .. } => {
                if *duration < 0.0 {
                    return Err(anyhow!(
                        "Action {}: wait duration cannot be negative",
                        index
                    ));
                }
            }
            ScriptAction::StartRecording { output_path, .. } => {
                if output_path.is_empty() {
                    return Err(anyhow!(
                        "Action {}: recording output path cannot be empty",
                        index
                    ));
                }
            }
            ScriptAction::SetVariable { name, .. } => {
                if name.is_empty() {
                    return Err(anyhow!("Action {}: variable name cannot be empty", index));
                }
            }
            _ => {} // Other actions are generally valid by structure
        }

        Ok(())
    }

    /// Execute a JSON DSL script
    pub async fn execute_script(
        &mut self,
        script: &JsonDslScript,
    ) -> Result<ScriptExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut execution_log = Vec::new();
        let mut actions_executed = 0u32;

        // Initialize execution context
        let mut variables = script.variables.clone().unwrap_or_default();
        if let Some(script_id) = &script.id {
            variables.insert(
                "script_id".to_string(),
                ScriptValue::String(script_id.clone()),
            );
        }
        variables.insert(
            "script_name".to_string(),
            ScriptValue::String(script.name.clone()),
        );

        let context = ExecutionContext {
            variables,
            state: ExecutionState::Running,
            call_stack: Vec::new(),
            start_time: Utc::now(),
        };

        self.context = Some(context);

        execution_log.push(format!("Starting script execution: {}", script.name));
        info!("Starting JSON DSL script execution: {}", script.name);

        let config = script.config.as_ref();
        let stop_on_error = config.and_then(|c| c.stop_on_error).unwrap_or(true);

        // Handle script-level looping
        let loop_count = config.and_then(|c| c.loop_count).unwrap_or(1);
        let loop_delay = config.and_then(|c| c.loop_delay).unwrap_or(0.0);

        for current_loop in 0..loop_count {
            if current_loop > 0 {
                execution_log.push(format!("Loop iteration {}", current_loop + 1));
                info!("Script loop iteration: {}", current_loop + 1);

                // Add loop delay between iterations
                if loop_delay > 0.0 {
                    tokio::time::sleep(Duration::from_secs_f64(loop_delay)).await;
                }
            }

            // Execute actions
            for (action_index, action) in script.actions.iter().enumerate() {
                match self.execute_action(action, action_index).await {
                    Ok(result) => {
                        actions_executed += 1;
                        if let Some(log_message) = result {
                            execution_log.push(log_message);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Action {} failed: {}", action_index, e);
                        execution_log.push(error_msg.clone());
                        error!("Script execution error: {}", error_msg);

                        if stop_on_error {
                            self.context.as_mut().unwrap().state =
                                ExecutionState::Failed(e.to_string());
                            return Ok(ScriptExecutionResult {
                                success: false,
                                error: Some(e.to_string()),
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                actions_executed,
                                final_variables: self.context.as_ref().unwrap().variables.clone(),
                                execution_log,
                            });
                        }
                    }
                }
            }
        }

        // Update context state
        if let Some(context) = &mut self.context {
            context.state = ExecutionState::Completed;
        }

        let execution_time = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!(
            "Script completed successfully. Executed {} actions in {}ms",
            actions_executed, execution_time
        ));
        info!("JSON DSL script completed successfully");

        Ok(ScriptExecutionResult {
            success: true,
            error: None,
            execution_time_ms: execution_time,
            actions_executed,
            final_variables: self.context.as_ref().unwrap().variables.clone(),
            execution_log,
        })
    }

    /// Execute a single action
    async fn execute_action(
        &mut self,
        action: &ScriptAction,
        action_index: usize,
    ) -> Result<Option<String>> {
        let context = self
            .context
            .as_mut()
            .ok_or_else(|| anyhow!("No execution context"))?;

        match action {
            ScriptAction::PlayVideo {
                path,
                start_time,
                duration,
                loop_video,
                device,
                volume,
            } => {
                let log_msg = format!(
                    "Playing video: {} (start: {:?}, duration: {:?}, loop: {:?})",
                    path, start_time, duration, loop_video
                );
                info!("{}", log_msg);

                // Integration with existing webcam streaming would go here
                // For now, we'll simulate the action
                if let Some(volume) = volume {
                    debug!("Setting video volume to: {}", volume);
                }

                Ok(Some(log_msg))
            }

            ScriptAction::PlayAudio {
                path,
                start_time,
                duration,
                loop_audio,
                device,
                volume,
            } => {
                let log_msg = format!(
                    "Playing audio: {} (start: {:?}, duration: {:?}, loop: {:?})",
                    path, start_time, duration, loop_audio
                );
                info!("{}", log_msg);

                if let Some(volume) = volume {
                    debug!("Setting audio volume to: {}", volume);
                }

                Ok(Some(log_msg))
            }

            ScriptAction::StopMedia { media_type, device } => {
                let log_msg = format!("Stopping media: type={:?}, device={:?}", media_type, device);
                info!("{}", log_msg);
                Ok(Some(log_msg))
            }

            ScriptAction::Wait {
                duration,
                description,
            } => {
                let desc = description.as_deref().unwrap_or("Waiting");
                let log_msg = format!("{} for {:.2} seconds", desc, duration);
                info!("{}", log_msg);

                tokio::time::sleep(Duration::from_secs_f64(*duration)).await;

                Ok(Some(log_msg))
            }

            ScriptAction::StartRecording {
                output_path,
                quality,
                record_video,
                record_audio,
                max_duration,
            } => {
                let log_msg = format!(
                    "Starting recording: {} (quality: {:?}, video: {:?}, audio: {:?})",
                    output_path, quality, record_video, record_audio
                );
                info!("{}", log_msg);
                Ok(Some(log_msg))
            }

            ScriptAction::StopRecording { save_path } => {
                let log_msg = format!("Stopping recording: save_path={:?}", save_path);
                info!("{}", log_msg);
                Ok(Some(log_msg))
            }

            ScriptAction::SetVirtualDevice {
                device_type,
                action,
                source,
            } => {
                let log_msg = format!(
                    "Virtual device {}: {} (source: {:?})",
                    device_type, action, source
                );
                info!("{}", log_msg);
                Ok(Some(log_msg))
            }

            ScriptAction::ExecuteCommand {
                command,
                args,
                working_dir,
                timeout,
            } => {
                let args_str = args.as_ref().map(|a| a.join(" ")).unwrap_or_default();
                let log_msg = format!("Executing command: {} {}", command, args_str);
                info!("{}", log_msg);

                // Command execution would go here with proper safety checks
                Ok(Some(log_msg))
            }

            ScriptAction::SetVariable { name, value } => {
                context.variables.insert(name.clone(), value.clone());
                let log_msg = format!("Set variable '{}' to {:?}", name, value);
                debug!("{}", log_msg);
                Ok(Some(log_msg))
            }

            ScriptAction::If {
                condition,
                then_actions,
                else_actions,
            } => {
                let condition_result = self.evaluate_condition(condition)?;
                let log_msg = format!("Condition evaluated to: {}", condition_result);
                debug!("{}", log_msg);

                let actions_to_execute = if condition_result {
                    then_actions
                } else {
                    else_actions.as_deref().unwrap_or(&[])
                };

                for (nested_index, nested_action) in actions_to_execute.iter().enumerate() {
                    self.execute_action(nested_action, nested_index).await?;
                }

                Ok(Some(log_msg))
            }

            ScriptAction::While {
                condition,
                actions,
                max_iterations,
            } => {
                let mut iteration_count = 0;
                let mut condition_result = self.evaluate_condition(condition)?;

                while condition_result {
                    if let Some(max_iter) = max_iterations {
                        if iteration_count >= *max_iter {
                            break;
                        }
                    }

                    for (nested_index, nested_action) in actions.iter().enumerate() {
                        self.execute_action(nested_action, nested_index).await?;
                    }

                    iteration_count += 1;
                    condition_result = self.evaluate_condition(condition)?;
                }

                let log_msg = format!("While loop completed after {} iterations", iteration_count);
                debug!("{}", log_msg);
                Ok(Some(log_msg))
            }

            ScriptAction::For {
                variable,
                from,
                to,
                step,
                actions,
            } => {
                let step_val = step.unwrap_or(1);
                let log_msg = format!(
                    "For loop: {} from {} to {} step {}",
                    variable, from, to, step_val
                );
                debug!("{}", log_msg);

                for i in (*from..*to).step_by(step_val.unsigned_abs() as usize) {
                    context
                        .variables
                        .insert(variable.clone(), ScriptValue::Integer(i as i64));

                    for (nested_index, nested_action) in actions.iter().enumerate() {
                        self.execute_action(nested_action, nested_index).await?;
                    }
                }

                Ok(Some(log_msg))
            }

            ScriptAction::Log {
                level,
                message,
                include_variables,
            } => {
                let level_str = level.as_deref().unwrap_or("info");
                let final_message = if include_variables.unwrap_or(false) {
                    self.format_message_with_variables(message)
                } else {
                    message.clone()
                };

                match level_str {
                    "error" => error!("{}", final_message),
                    "warn" => warn!("{}", final_message),
                    "debug" => debug!("{}", final_message),
                    _ => info!("{}", final_message),
                }

                Ok(Some(format!(
                    "[{}] {}",
                    level_str.to_uppercase(),
                    final_message
                )))
            }

            ScriptAction::CallFunction {
                function,
                parameters,
            } => {
                let log_msg = format!(
                    "Calling function: {} with parameters: {:?}",
                    function, parameters
                );
                info!("{}", log_msg);

                // Function calling would integrate with the existing Rhai engine
                Ok(Some(log_msg))
            }
        }
    }

    /// Evaluate a script condition
    fn evaluate_condition(&self, condition: &ScriptCondition) -> Result<bool> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| anyhow!("No execution context"))?;

        match condition {
            ScriptCondition::Equals { left, right } => {
                Ok(self.compare_values(left, right) == std::cmp::Ordering::Equal)
            }
            ScriptCondition::NotEquals { left, right } => {
                Ok(self.compare_values(left, right) != std::cmp::Ordering::Equal)
            }
            ScriptCondition::GreaterThan { left, right } => {
                Ok(self.compare_values(left, right) == std::cmp::Ordering::Greater)
            }
            ScriptCondition::LessThan { left, right } => {
                Ok(self.compare_values(left, right) == std::cmp::Ordering::Less)
            }
            ScriptCondition::GreaterThanOrEqual { left, right } => {
                let cmp = self.compare_values(left, right);
                Ok(cmp == std::cmp::Ordering::Greater || cmp == std::cmp::Ordering::Equal)
            }
            ScriptCondition::LessThanOrEqual { left, right } => {
                let cmp = self.compare_values(left, right);
                Ok(cmp == std::cmp::Ordering::Less || cmp == std::cmp::Ordering::Equal)
            }
            ScriptCondition::And { conditions } => {
                for cond in conditions {
                    if !self.evaluate_condition(cond)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ScriptCondition::Or { conditions } => {
                for cond in conditions {
                    if self.evaluate_condition(cond)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ScriptCondition::Not { condition } => Ok(!self.evaluate_condition(condition)?),
            ScriptCondition::VariableExists { name } => Ok(context.variables.contains_key(name)),
            ScriptCondition::FileExists { path } => Ok(std::path::Path::new(path).exists()),
        }
    }

    /// Compare two script values
    fn compare_values(&self, left: &ScriptValue, right: &ScriptValue) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (left, right) {
            (ScriptValue::String(l), ScriptValue::String(r)) => l.cmp(r),
            (ScriptValue::Number(l), ScriptValue::Number(r)) => {
                l.partial_cmp(r).unwrap_or(Ordering::Equal)
            }
            (ScriptValue::Integer(l), ScriptValue::Integer(r)) => l.cmp(r),
            (ScriptValue::Boolean(l), ScriptValue::Boolean(r)) => l.cmp(r),
            (ScriptValue::Integer(l), ScriptValue::Number(r)) => {
                (*l as f64).partial_cmp(r).unwrap_or(Ordering::Equal)
            }
            (ScriptValue::Number(l), ScriptValue::Integer(r)) => {
                l.partial_cmp(&(*r as f64)).unwrap_or(Ordering::Equal)
            }
            _ => Ordering::Equal, // Default fallback
        }
    }

    /// Format message with variable substitution
    fn format_message_with_variables(&self, message: &str) -> String {
        let context = self.context.as_ref().unwrap();
        let mut result = message.to_string();

        for (name, value) in &context.variables {
            let placeholder = format!("${{{}}}", name);
            let replacement = match value {
                ScriptValue::String(s) => s.clone(),
                ScriptValue::Number(n) => n.to_string(),
                ScriptValue::Integer(i) => i.to_string(),
                ScriptValue::Boolean(b) => b.to_string(),
                ScriptValue::Null => "null".to_string(),
                ScriptValue::Array(_) => "[array]".to_string(),
                ScriptValue::Object(_) => "[object]".to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }

        result
    }

    /// Get current execution context
    pub fn get_context(&self) -> Option<&ExecutionContext> {
        self.context.as_ref()
    }

    /// Stop current script execution
    pub fn stop_execution(&mut self) -> Result<()> {
        if let Some(context) = &mut self.context {
            context.state = ExecutionState::Cancelled;
            info!("Script execution cancelled");
            Ok(())
        } else {
            Err(anyhow!("No script execution in progress"))
        }
    }
}

impl Default for JsonDslEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for JsonDslScript {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            description: None,
            version: None,
            metadata: None,
            actions: Vec::new(),
            variables: None,
            config: None,
        }
    }
}

impl Default for ScriptMetadata {
    fn default() -> Self {
        Self {
            author: None,
            created_at: None,
            modified_at: None,
            tags: None,
            estimated_duration: None,
        }
    }
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            stop_on_error: Some(true),
            default_timeout: Some(30.0),
            loop_script: Some(false),
            loop_count: Some(1),
            loop_delay: Some(0.0),
        }
    }
}

/// Create example scripts for testing
pub fn create_example_scripts() -> Vec<JsonDslScript> {
    vec![
        // Simple video playback script
        JsonDslScript {
            id: Some("example_video_playback".to_string()),
            name: "Simple Video Playback".to_string(),
            description: Some("Play a video file for 10 seconds".to_string()),
            version: Some("1.0.0".to_string()),
            metadata: Some(ScriptMetadata {
                author: Some("System".to_string()),
                created_at: Some(Utc::now()),
                modified_at: Some(Utc::now()),
                tags: Some(vec!["video".to_string(), "simple".to_string()]),
                estimated_duration: Some(10.0),
            }),
            actions: vec![
                ScriptAction::Log {
                    level: Some("info".to_string()),
                    message: "Starting video playback example".to_string(),
                    include_variables: Some(false),
                },
                ScriptAction::PlayVideo {
                    path: "example.mp4".to_string(),
                    start_time: Some(0.0),
                    duration: Some(10.0),
                    loop_video: Some(false),
                    device: Some("webcam".to_string()),
                    volume: Some(0.8),
                },
                ScriptAction::Wait {
                    duration: 10.0,
                    description: Some("Waiting for video to complete".to_string()),
                },
                ScriptAction::StopMedia {
                    media_type: Some("video".to_string()),
                    device: None,
                },
                ScriptAction::Log {
                    level: Some("info".to_string()),
                    message: "Video playback completed".to_string(),
                    include_variables: Some(false),
                },
            ],
            variables: None,
            config: Some(ScriptConfig {
                stop_on_error: Some(true),
                default_timeout: Some(30.0),
                loop_script: Some(false),
                loop_count: Some(1),
                loop_delay: Some(0.0),
            }),
        },
        // Complex media sequence with conditional logic
        JsonDslScript {
            id: Some("complex_media_sequence".to_string()),
            name: "Complex Media Sequence".to_string(),
            description: Some("Advanced script with conditional logic and looping".to_string()),
            version: Some("1.0.0".to_string()),
            metadata: Some(ScriptMetadata {
                author: Some("System".to_string()),
                created_at: Some(Utc::now()),
                modified_at: Some(Utc::now()),
                tags: Some(vec![
                    "complex".to_string(),
                    "media".to_string(),
                    "looping".to_string(),
                ]),
                estimated_duration: Some(60.0),
            }),
            actions: vec![
                ScriptAction::SetVariable {
                    name: "loop_count".to_string(),
                    value: ScriptValue::Integer(0),
                },
                ScriptAction::Log {
                    level: Some("info".to_string()),
                    message: "Starting complex media sequence".to_string(),
                    include_variables: Some(false),
                },
                ScriptAction::If {
                    condition: ScriptCondition::FileExists {
                        path: "background_music.mp3".to_string(),
                    },
                    then_actions: vec![
                        ScriptAction::PlayAudio {
                            path: "background_music.mp3".to_string(),
                            start_time: Some(0.0),
                            duration: None,
                            loop_audio: Some(true),
                            device: Some("speaker".to_string()),
                            volume: Some(0.5),
                        },
                        ScriptAction::Log {
                            level: Some("info".to_string()),
                            message: "Background music started".to_string(),
                            include_variables: Some(false),
                        },
                    ],
                    else_actions: Some(vec![ScriptAction::Log {
                        level: Some("warn".to_string()),
                        message: "Background music file not found".to_string(),
                        include_variables: Some(false),
                    }]),
                },
                ScriptAction::For {
                    variable: "i".to_string(),
                    from: 0,
                    to: 3,
                    step: Some(1),
                    actions: vec![
                        ScriptAction::SetVariable {
                            name: "loop_count".to_string(),
                            value: ScriptValue::Integer(1),
                        },
                        ScriptAction::PlayVideo {
                            path: "video_segment.mp4".to_string(),
                            start_time: Some(0.0),
                            duration: Some(5.0),
                            loop_video: Some(false),
                            device: Some("webcam".to_string()),
                            volume: Some(0.8),
                        },
                        ScriptAction::Wait {
                            duration: 5.0,
                            description: Some("Video segment duration".to_string()),
                        },
                        ScriptAction::Log {
                            level: Some("info".to_string()),
                            message: "Completed video iteration ${loop_count}".to_string(),
                            include_variables: Some(true),
                        },
                    ],
                },
                ScriptAction::StopMedia {
                    media_type: Some("all".to_string()),
                    device: None,
                },
                ScriptAction::Log {
                    level: Some("info".to_string()),
                    message: "Complex media sequence completed".to_string(),
                    include_variables: Some(true),
                },
            ],
            variables: Some(HashMap::from([(
                "total_loops".to_string(),
                ScriptValue::Integer(3),
            )])),
            config: Some(ScriptConfig {
                stop_on_error: Some(true),
                default_timeout: Some(60.0),
                loop_script: Some(false),
                loop_count: Some(1),
                loop_delay: Some(0.0),
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_parse_simple_script() {
        let engine = JsonDslEngine::new();

        let script_json = r#"
        {
            "name": "Test Script",
            "description": "A simple test script",
            "actions": [
                {
                    "type": "log",
                    "level": "info",
                    "message": "Hello, World!"
                },
                {
                    "type": "wait",
                    "duration": 1.0,
                    "description": "Test wait"
                }
            ]
        }
        "#;

        let script = engine.parse_script(script_json).unwrap();
        assert_eq!(script.name, "Test Script");
        assert_eq!(script.actions.len(), 2);
    }

    #[test]
    fn test_script_validation() {
        let engine = JsonDslEngine::new();

        // Test invalid script (empty name)
        let invalid_json = r#"
        {
            "name": "",
            "actions": []
        }
        "#;

        let result = engine.parse_script(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_condition_evaluation() {
        let engine = JsonDslEngine::new();

        let condition = ScriptCondition::Equals {
            left: ScriptValue::Integer(5),
            right: ScriptValue::Integer(5),
        };

        assert_eq!(engine.evaluate_condition(&condition).unwrap(), true);
    }

    #[tokio::test]
    async fn test_simple_execution() {
        let mut engine = JsonDslEngine::new();

        let script = JsonDslScript {
            name: "Test Execution".to_string(),
            actions: vec![
                ScriptAction::SetVariable {
                    name: "test_var".to_string(),
                    value: ScriptValue::String("test_value".to_string()),
                },
                ScriptAction::Log {
                    level: Some("info".to_string()),
                    message: "Test execution".to_string(),
                    include_variables: Some(false),
                },
            ],
            ..Default::default()
        };

        let result = engine.execute_script(&script).await.unwrap();
        assert!(result.success);
        assert_eq!(result.actions_executed, 2);
    }

    #[test]
    fn test_example_scripts() {
        let examples = create_example_scripts();
        assert_eq!(examples.len(), 2);

        let engine = JsonDslEngine::new();
        for script in &examples {
            assert!(engine.validate_script(script).is_ok());
        }
    }
}
