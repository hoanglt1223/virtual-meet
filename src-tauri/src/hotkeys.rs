//! Hotkey Management Module
//!
//! This module handles global hotkey registration, management, and execution.

use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, error, warn, debug};

/// Hotkey manager for handling global keyboard shortcuts
pub struct HotkeyManager {
    registered_hotkeys: HashMap<String, Hotkey>,
    hotkey_callbacks: HashMap<String, Box<dyn Fn() + Send + Sync>>,
}

/// Hotkey definition
#[derive(Debug, Clone)]
pub struct Hotkey {
    pub id: String,
    pub name: String,
    pub key_combination: String,
    pub description: String,
    pub enabled: bool,
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyManager {
    /// Create a new hotkey manager
    pub fn new() -> Self {
        Self {
            registered_hotkeys: HashMap::new(),
            hotkey_callbacks: HashMap::new(),
        }
    }

    /// Register a new hotkey
    pub fn register_hotkey<F>(&mut self, hotkey: Hotkey, callback: F) -> Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        info!("Registering hotkey: {} ({})", hotkey.name, hotkey.key_combination);

        // Check for conflicts
        if self.has_conflict(&hotkey.key_combination, &hotkey.id) {
            return Err(anyhow::anyhow!(
                "Hotkey combination '{}' conflicts with existing hotkey",
                hotkey.key_combination
            ));
        }

        // Register with the system (simplified implementation)
        // In a real implementation, you would use the global-hotkey crate

        self.registered_hotkeys.insert(hotkey.id.clone(), hotkey);
        self.hotkey_callbacks.insert(hotkey.id.clone(), Box::new(callback));

        Ok(())
    }

    /// Unregister a hotkey
    pub fn unregister_hotkey(&mut self, hotkey_id: &str) -> Result<()> {
        info!("Unregistering hotkey: {}", hotkey_id);

        self.registered_hotkeys.remove(hotkey_id);
        self.hotkey_callbacks.remove(hotkey_id);

        // Unregister from the system (simplified implementation)

        Ok(())
    }

    /// Check if a key combination conflicts with existing hotkeys
    pub fn has_conflict(&self, key_combination: &str, exclude_id: &str) -> bool {
        self.registered_hotkeys
            .iter()
            .any(|(id, hotkey)| id != exclude_id && hotkey.key_combination == key_combination)
    }

    /// Get all registered hotkeys
    pub fn get_hotkeys(&self) -> Vec<&Hotkey> {
        self.registered_hotkeys.values().collect()
    }

    /// Execute a hotkey by ID
    pub fn execute_hotkey(&self, hotkey_id: &str) -> Result<()> {
        if let Some(callback) = self.hotkey_callbacks.get(hotkey_id) {
            debug!("Executing hotkey: {}", hotkey_id);
            callback();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Hotkey not found: {}", hotkey_id))
        }
    }

    /// Enable/disable a hotkey
    pub fn set_hotkey_enabled(&mut self, hotkey_id: &str, enabled: bool) -> Result<()> {
        if let Some(hotkey) = self.registered_hotkeys.get_mut(hotkey_id) {
            hotkey.enabled = enabled;
            info!("Hotkey '{}' {}", hotkey_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(anyhow::anyhow!("Hotkey not found: {}", hotkey_id))
        }
    }
}

/// Validate a key combination format
pub fn validate_key_combination(combination: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_manager() {
        let mut manager = HotkeyManager::new();
        let hotkey = Hotkey {
            id: "test".to_string(),
            name: "Test Hotkey".to_string(),
            key_combination: "Ctrl+Shift+T".to_string(),
            description: "Test hotkey".to_string(),
            enabled: true,
        };

        // Register hotkey
        assert!(manager.register_hotkey(hotkey.clone(), || println!("Hotkey triggered!")).is_ok());

        // Check for conflicts
        let conflict_hotkey = Hotkey {
            id: "conflict".to_string(),
            name: "Conflict Hotkey".to_string(),
            key_combination: "Ctrl+Shift+T".to_string(),
            description: "Conflicting hotkey".to_string(),
            enabled: true,
        };
        assert!(manager.register_hotkey(conflict_hotkey, || println!("Should not trigger")).is_err());

        // Execute hotkey
        assert!(manager.execute_hotkey("test").is_ok());
        assert!(manager.execute_hotkey("nonexistent").is_err());
    }

    #[test]
    fn test_validate_key_combination() {
        assert!(validate_key_combination("Ctrl+Shift+V"));
        assert!(validate_key_combination("Alt+F4"));
        assert!(validate_key_combination("Ctrl+Space"));
        assert!(validate_key_combination("Win+UpArrow"));

        assert!(!validate_key_combination("Ctrl"));
        assert!(!validate_key_combination("V"));
        assert!(!validate_key_combination(""));
    }
}