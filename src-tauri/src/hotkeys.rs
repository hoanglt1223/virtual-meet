//! Hotkey Management Module
//!
//! This module handles global hotkey registration, management, and execution.

use anyhow::Result;
use global_hotkey::{
    hotkey::{Code, Modifiers},
    GlobalHotkeyManager, Hotkey,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Enhanced hotkey definition with global registration support
#[derive(Debug, Clone)]
pub struct HotkeyDefinition {
    pub id: String,
    pub name: String,
    pub key_combination: String,
    pub description: String,
    pub enabled: bool,
    pub global: bool,
    pub modifiers: Option<Modifiers>,
    pub key_code: Option<Code>,
}

/// Hotkey manager for handling global keyboard shortcuts
pub struct HotkeyManager {
    registered_hotkeys: HashMap<String, HotkeyDefinition>,
    hotkey_callbacks: HashMap<String, Arc<dyn Fn() + Send + Sync>>,
    global_manager: Option<GlobalHotkeyManager>,
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
            global_manager: GlobalHotkeyManager::new().ok(),
        }
    }

    /// Register a new hotkey
    pub fn register_hotkey<F>(&mut self, hotkey: HotkeyDefinition, callback: F) -> Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        info!(
            "Registering hotkey: {} ({})",
            hotkey.name, hotkey.key_combination
        );

        // Check for conflicts
        if self.has_conflict(&hotkey.key_combination, &hotkey.id) {
            return Err(anyhow::anyhow!(
                "Hotkey combination '{}' conflicts with existing hotkey",
                hotkey.key_combination
            ));
        }

        // Parse key combination for global registration
        let global_hotkey = self.parse_key_combination(&hotkey.key_combination)?;

        // Extract modifiers and key code for storage
        let (modifiers, key_code) = match global_hotkey {
            Hotkey { mods, key } => (Some(mods), Some(key)),
        };

        // Create enhanced hotkey definition with parsed data
        let mut enhanced_hotkey = hotkey.clone();
        enhanced_hotkey.modifiers = modifiers;
        enhanced_hotkey.key_code = key_code;

        // Register with the OS if it's a global hotkey
        if hotkey.global {
            if let Some(ref mut manager) = self.global_manager {
                manager.register(global_hotkey)?;
                info!(
                    "Globally registered hotkey: {} ({})",
                    hotkey.name, hotkey.key_combination
                );
            } else {
                warn!("Global hotkey manager not available, hotkey will be local only");
            }
        }

        self.registered_hotkeys
            .insert(hotkey.id.clone(), enhanced_hotkey);
        self.hotkey_callbacks
            .insert(hotkey.id.clone(), Arc::new(callback));

        Ok(())
    }

    /// Unregister a hotkey
    pub fn unregister_hotkey(&mut self, hotkey_id: &str) -> Result<()> {
        info!("Unregistering hotkey: {}", hotkey_id);

        if let Some(hotkey) = self.registered_hotkeys.get(hotkey_id) {
            // Unregister from OS if it was global
            if hotkey.global {
                if let (Some(ref mut manager), Some(key_code)) =
                    (&mut self.global_manager, hotkey.key_code)
                {
                    let modifiers = hotkey.modifiers.unwrap_or(Modifiers::NONE);
                    let global_hotkey = Hotkey::new(modifiers, key_code);
                    if let Err(e) = manager.unregister(global_hotkey) {
                        warn!("Failed to unregister global hotkey: {}", e);
                    }
                }
            }
        }

        self.registered_hotkeys.remove(hotkey_id);
        self.hotkey_callbacks.remove(hotkey_id);

        Ok(())
    }

    /// Check if a key combination conflicts with existing hotkeys
    pub fn has_conflict(&self, key_combination: &str, exclude_id: &str) -> bool {
        self.registered_hotkeys
            .iter()
            .any(|(id, hotkey)| id != exclude_id && hotkey.key_combination == key_combination)
    }

    /// Get all registered hotkeys
    pub fn get_hotkeys(&self) -> Vec<&HotkeyDefinition> {
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
            info!(
                "Hotkey '{}' {}",
                hotkey_id,
                if enabled { "enabled" } else { "disabled" }
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Hotkey not found: {}", hotkey_id))
        }
    }

    /// Parse a key combination string into a global hotkey
    fn parse_key_combination(&self, combination: &str) -> Result<Hotkey> {
        let parts: Vec<&str> = combination.to_uppercase().split('+').collect();

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty key combination"));
        }

        let mut modifiers = Modifiers::NONE;
        let mut key_code: Option<Code> = None;

        for part in parts {
            match part.trim() {
                "CTRL" | "CONTROL" => modifiers |= Modifiers::CONTROL,
                "ALT" => modifiers |= Modifiers::ALT,
                "SHIFT" => modifiers |= Modifiers::SHIFT,
                "WIN" | "CMD" | "META" | "SUPER" => modifiers |= Modifiers::SUPER,
                "F1" => key_code = Some(Code::F1),
                "F2" => key_code = Some(Code::F2),
                "F3" => key_code = Some(Code::F3),
                "F4" => key_code = Some(Code::F4),
                "F5" => key_code = Some(Code::F5),
                "F6" => key_code = Some(Code::F6),
                "F7" => key_code = Some(Code::F7),
                "F8" => key_code = Some(Code::F8),
                "F9" => key_code = Some(Code::F9),
                "F10" => key_code = Some(Code::F10),
                "F11" => key_code = Some(Code::F11),
                "F12" => key_code = Some(Code::F12),
                "A" => key_code = Some(Code::KeyA),
                "B" => key_code = Some(Code::KeyB),
                "C" => key_code = Some(Code::KeyC),
                "D" => key_code = Some(Code::KeyD),
                "E" => key_code = Some(Code::KeyE),
                "F" => key_code = Some(Code::KeyF),
                "G" => key_code = Some(Code::KeyG),
                "H" => key_code = Some(Code::KeyH),
                "I" => key_code = Some(Code::KeyI),
                "J" => key_code = Some(Code::KeyJ),
                "K" => key_code = Some(Code::KeyK),
                "L" => key_code = Some(Code::KeyL),
                "M" => key_code = Some(Code::KeyM),
                "N" => key_code = Some(Code::KeyN),
                "O" => key_code = Some(Code::KeyO),
                "P" => key_code = Some(Code::KeyP),
                "Q" => key_code = Some(Code::KeyQ),
                "R" => key_code = Some(Code::KeyR),
                "S" => key_code = Some(Code::KeyS),
                "T" => key_code = Some(Code::KeyT),
                "U" => key_code = Some(Code::KeyU),
                "V" => key_code = Some(Code::KeyV),
                "W" => key_code = Some(Code::KeyW),
                "X" => key_code = Some(Code::KeyX),
                "Y" => key_code = Some(Code::KeyY),
                "Z" => key_code = Some(Code::KeyZ),
                "0" => key_code = Some(Code::Digit0),
                "1" => key_code = Some(Code::Digit1),
                "2" => key_code = Some(Code::Digit2),
                "3" => key_code = Some(Code::Digit3),
                "4" => key_code = Some(Code::Digit4),
                "5" => key_code = Some(Code::Digit5),
                "6" => key_code = Some(Code::Digit6),
                "7" => key_code = Some(Code::Digit7),
                "8" => key_code = Some(Code::Digit8),
                "9" => key_code = Some(Code::Digit9),
                "SPACE" => key_code = Some(Code::Space),
                "ENTER" | "RETURN" => key_code = Some(Code::Enter),
                "ESCAPE" | "ESC" => key_code = Some(Code::Escape),
                "TAB" => key_code = Some(Code::Tab),
                "BACKSPACE" => key_code = Some(Code::Backspace),
                "DELETE" | "DEL" => key_code = Some(Code::Delete),
                "INSERT" => key_code = Some(Code::Insert),
                "HOME" => key_code = Some(Code::Home),
                "END" => key_code = Some(Code::End),
                "PAGEUP" => key_code = Some(Code::PageUp),
                "PAGEDOWN" => key_code = Some(Code::PageDown),
                "UP" | "UPARROW" => key_code = Some(Code::ArrowUp),
                "DOWN" | "DOWNARROW" => key_code = Some(Code::ArrowDown),
                "LEFT" | "LEFTARROW" => key_code = Some(Code::ArrowLeft),
                "RIGHT" | "RIGHTARROW" => key_code = Some(Code::ArrowRight),
                _ => return Err(anyhow::anyhow!("Unsupported key or modifier: {}", part)),
            }
        }

        match key_code {
            Some(code) => Ok(Hotkey::new(modifiers, code)),
            None => Err(anyhow::anyhow!(
                "No valid key found in combination: {}",
                combination
            )),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_manager() {
        let mut manager = HotkeyManager::new();
        let hotkey = HotkeyDefinition {
            id: "test".to_string(),
            name: "Test Hotkey".to_string(),
            key_combination: "Ctrl+Shift+T".to_string(),
            description: "Test hotkey".to_string(),
            enabled: true,
            global: false,
            modifiers: None,
            key_code: None,
        };

        // Register hotkey
        assert!(manager
            .register_hotkey(hotkey.clone(), || println!("Hotkey triggered!"))
            .is_ok());

        // Check for conflicts
        let conflict_hotkey = HotkeyDefinition {
            id: "conflict".to_string(),
            name: "Conflict Hotkey".to_string(),
            key_combination: "Ctrl+Shift+T".to_string(),
            description: "Conflicting hotkey".to_string(),
            enabled: true,
            global: false,
            modifiers: None,
            key_code: None,
        };
        assert!(manager
            .register_hotkey(conflict_hotkey, || println!("Should not trigger"))
            .is_err());

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
        assert!(validate_key_combination("Ctrl+F1"));
        assert!(validate_key_combination("Alt+F12"));

        assert!(!validate_key_combination("Ctrl"));
        assert!(!validate_key_combination("V"));
        assert!(!validate_key_combination(""));
        assert!(!validate_key_combination("F1")); // Function keys alone need modifier
    }

    #[test]
    fn test_parse_key_combination() {
        let manager = HotkeyManager::new();

        // Test F1-F12 parsing
        assert!(manager.parse_key_combination("Ctrl+F1").is_ok());
        assert!(manager.parse_key_combination("Alt+F12").is_ok());
        assert!(manager.parse_key_combination("Shift+F5").is_ok());
        assert!(manager.parse_key_combination("Ctrl+Shift+F10").is_ok());

        // Test regular keys
        assert!(manager.parse_key_combination("Ctrl+V").is_ok());
        assert!(manager.parse_key_combination("Alt+Tab").is_ok());
        assert!(manager.parse_key_combination("Win+Space").is_ok());

        // Test invalid combinations
        assert!(manager.parse_key_combination("F1").is_err()); // No modifier
        assert!(manager.parse_key_combination("Ctrl+").is_err()); // Missing key
        assert!(manager.parse_key_combination("Invalid+F1").is_err());
    }
}
