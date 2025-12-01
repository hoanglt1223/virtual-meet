//! Scripting Module
//!
//! This module handles script execution, management, and automation using the Rhai scripting engine.

use anyhow::Result;
use rhai::{Engine, Scope, Dynamic, FuncArgs};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, error, warn, debug};

/// Script execution engine
pub struct ScriptEngine {
    engine: Engine,
    global_scope: Scope,
    scripts: HashMap<String, Script>,
}

/// Script definition
#[derive(Debug, Clone)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub content: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Script execution result
#[derive(Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine {
    /// Create a new script engine
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Register custom functions and types
        Self::register_functions(&mut engine);

        Self {
            engine,
            global_scope: Scope::new(),
            scripts: HashMap::new(),
        }
    }

    /// Register custom functions with the Rhai engine
    fn register_functions(engine: &mut Engine) {
        // Register logging functions
        engine.register_fn("print", |msg: &str| {
            info!("Script output: {}", msg);
        });

        engine.register_fn("debug", |msg: &str| {
            debug!("Script debug: {}", msg);
        });

        engine.register_fn("warn", |msg: &str| {
            warn!("Script warning: {}", msg);
        });

        engine.register_fn("error", |msg: &str| {
            error!("Script error: {}", msg);
        });

        // Register utility functions
        engine.register_fn("sleep", |seconds: i64| {
            std::thread::sleep(std::time::Duration::from_secs(seconds as u64));
        });

        engine.register_fn("timestamp", || {
            chrono::Utc::now().timestamp()
        });

        engine.register_fn("format_duration", |seconds: i64| {
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            let secs = seconds % 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, secs)
        });

        // Register virtual device functions (stubs)
        engine.register_fn("start_webcam_streaming", |path: &str| {
            info!("Script: Starting webcam streaming: {}", path);
        });

        engine.register_fn("stop_webcam_streaming", || {
            info!("Script: Stopping webcam streaming");
        });

        engine.register_fn("start_microphone_streaming", |path: &str| {
            info!("Script: Starting microphone streaming: {}", path);
        });

        engine.register_fn("stop_microphone_streaming", || {
            info!("Script: Stopping microphone streaming");
        });

        // Register recording functions (stubs)
        engine.register_fn("start_recording", |path: &str| {
            info!("Script: Starting recording: {}", path);
        });

        engine.register_fn("stop_recording", || {
            info!("Script: Stopping recording");
        });
    }

    /// Load a script
    pub fn load_script(&mut self, script: Script) -> Result<()> {
        // Validate script syntax
        if let Err(e) = self.engine.compile(&script.content) {
            return Err(anyhow::anyhow!("Script compilation error: {}", e));
        }

        self.scripts.insert(script.id.clone(), script);
        Ok(())
    }

    /// Execute a script by ID
    pub fn execute_script(&mut self, script_id: &str, variables: Option<HashMap<String, Dynamic>>) -> ExecutionResult {
        let start_time = std::time::Instant::now();

        let script = match self.scripts.get(script_id) {
            Some(script) if script.enabled => script.clone(),
            Some(_) => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some("Script is disabled".to_string()),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                };
            },
            None => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Script not found: {}", script_id)),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        info!("Executing script: {} ({})", script.name, script_id);

        // Create a new scope for this execution
        let mut scope = self.global_scope.clone();

        // Add variables to scope
        if let Some(vars) = variables {
            for (key, value) in vars {
                scope.push_dynamic(key, value);
            }
        }

        // Execute the script
        match self.engine.eval_with_scope::<Dynamic>(&mut scope, &script.content) {
            Ok(result) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                info!("Script executed successfully: {} ({}ms)", script.name, execution_time);

                ExecutionResult {
                    success: true,
                    output: result.to_string(),
                    error: None,
                    execution_time_ms: execution_time,
                }
            },
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                error!("Script execution failed: {} - {}", script.name, e);

                ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Execution error: {}", e)),
                    execution_time_ms: execution_time,
                }
            }
        }
    }

    /// Execute script content directly
    pub fn execute_content(&mut self, content: &str, variables: Option<HashMap<String, Dynamic>>) -> ExecutionResult {
        let start_time = std::time::Instant::now();

        info!("Executing script content");

        // Create a new scope for this execution
        let mut scope = self.global_scope.clone();

        // Add variables to scope
        if let Some(vars) = variables {
            for (key, value) in vars {
                scope.push_dynamic(key, value);
            }
        }

        // Execute the script
        match self.engine.eval_with_scope::<Dynamic>(&mut scope, content) {
            Ok(result) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                ExecutionResult {
                    success: true,
                    output: result.to_string(),
                    error: None,
                    execution_time_ms: execution_time,
                }
            },
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Execution error: {}", e)),
                    execution_time_ms: execution_time,
                }
            }
        }
    }

    /// Validate script syntax
    pub fn validate_syntax(&self, content: &str) -> Result<()> {
        self.engine.compile(content)?;
        Ok(())
    }

    /// Get all loaded scripts
    pub fn get_scripts(&self) -> Vec<&Script> {
        self.scripts.values().collect()
    }

    /// Remove a script
    pub fn remove_script(&mut self, script_id: &str) -> Option<Script> {
        self.scripts.remove(script_id)
    }

    /// Enable/disable a script
    pub fn set_script_enabled(&mut self, script_id: &str, enabled: bool) -> Result<()> {
        if let Some(script) = self.scripts.get_mut(script_id) {
            script.enabled = enabled;
            info!("Script '{}' {}", script_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(anyhow::anyhow!("Script not found: {}", script_id))
        }
    }
}

/// Create a script engine instance
pub fn create_script_engine() -> ScriptEngine {
    ScriptEngine::new()
}

/// Validate Rhai script syntax
pub fn validate_rhai_script(content: &str) -> Result<()> {
    let engine = Engine::new();
    engine.compile(content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_engine() {
        let mut engine = ScriptEngine::new();

        // Test valid script
        let valid_script = Script {
            id: "test".to_string(),
            name: "Test Script".to_string(),
            content: "print(\"Hello, World!\");".to_string(),
            enabled: true,
            created_at: chrono::Utc::now(),
        };

        assert!(engine.load_script(valid_script).is_ok());

        // Test script execution
        let result = engine.execute_script("test", None);
        assert!(result.success);

        // Test invalid script
        let invalid_script = Script {
            id: "invalid".to_string(),
            name: "Invalid Script".to_string(),
            content: "print(\"Hello, World!\"".to_string(), // Missing closing parenthesis
            enabled: true,
            created_at: chrono::Utc::now(),
        };

        assert!(engine.load_script(invalid_script).is_err());
    }

    #[test]
    fn test_script_validation() {
        assert!(validate_rhai_script("print(\"Hello\");").is_ok());
        assert!(validate_rhai_script("print(\"Hello\"").is_err()); // Syntax error
    }

    #[test]
    fn test_execute_content() {
        let mut engine = ScriptEngine::new();

        let result = engine.execute_content("let x = 42; print(x);", None);
        assert!(result.success);
    }

    #[test]
    fn test_engine_functions() {
        let mut engine = ScriptEngine::new();

        // Test print function
        let result = engine.execute_content("print(\"Test message\");", None);
        assert!(result.success);

        // Test timestamp function
        let result = engine.execute_content("let ts = timestamp(); print(ts);", None);
        assert!(result.success);

        // Test sleep function (will be very short)
        let result = engine.execute_content("sleep(0);", None);
        assert!(result.success);
    }
}