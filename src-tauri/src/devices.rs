//! Device Enumeration System
//!
//! This module provides comprehensive device discovery and enumeration capabilities
//! for both audio and video devices, with the ability to distinguish between
//! physical and virtual devices and detect their capabilities.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing::{debug, error, info, warn};

pub mod audio;
pub mod capabilities;
pub mod video;

use audio::AudioDeviceEnumerator;
use capabilities::DeviceCapabilityDetector;
use video::VideoDeviceEnumerator;

/// Device type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Audio,
    Video,
}

/// Device category enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceCategory {
    Input,
    Output,
    Both,
}

/// Device origin - distinguishes between physical and virtual devices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceOrigin {
    Physical, // Actual hardware devices
    Virtual,  // Software-based/virtual devices
    Unknown,  // Cannot determine origin
}

impl fmt::Display for DeviceOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceOrigin::Physical => write!(f, "Physical"),
            DeviceOrigin::Virtual => write!(f, "Virtual"),
            DeviceOrigin::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Common device information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Unique device identifier
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Device type (Audio/Video)
    pub device_type: DeviceType,
    /// Device category (Input/Output/Both)
    pub category: DeviceCategory,
    /// Device origin (Physical/Virtual)
    pub origin: DeviceOrigin,
    /// Device driver/provider name
    pub driver: String,
    /// Device description
    pub description: String,
    /// Whether the device is currently available
    pub is_available: bool,
    /// Whether the device is currently in use
    pub is_in_use: bool,
    /// Additional device-specific properties
    pub properties: HashMap<String, String>,
}

impl DeviceInfo {
    /// Create a new device info
    pub fn new(
        id: String,
        name: String,
        device_type: DeviceType,
        category: DeviceCategory,
        origin: DeviceOrigin,
    ) -> Self {
        Self {
            id,
            name,
            device_type,
            category,
            origin,
            driver: String::new(),
            description: String::new(),
            is_available: true,
            is_in_use: false,
            properties: HashMap::new(),
        }
    }

    /// Check if device is virtual
    pub fn is_virtual(&self) -> bool {
        matches!(self.origin, DeviceOrigin::Virtual)
    }

    /// Check if device is physical
    pub fn is_physical(&self) -> bool {
        matches!(self.origin, DeviceOrigin::Physical)
    }

    /// Get device display name with origin indicator
    pub fn display_name(&self) -> String {
        match self.origin {
            DeviceOrigin::Virtual => format!("{} [Virtual]", self.name),
            DeviceOrigin::Physical => self.name.clone(),
            DeviceOrigin::Unknown => format!("{} [Unknown]", self.name),
        }
    }

    /// Add a property to the device
    pub fn add_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Get a device property
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }
}

/// Device capability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Supported formats
    pub supported_formats: Vec<String>,
    /// Supported resolutions (for video devices)
    pub supported_resolutions: Vec<(u32, u32)>,
    /// Supported frame rates (for video devices)
    pub supported_frame_rates: Vec<f32>,
    /// Supported sample rates (for audio devices)
    pub supported_sample_rates: Vec<u32>,
    /// Supported channel counts (for audio devices)
    pub supported_channel_counts: Vec<u32>,
    /// Minimum and maximum buffer sizes
    pub buffer_size_range: Option<(usize, usize)>,
    /// Additional capability flags
    pub capability_flags: Vec<String>,
}

impl DeviceCapabilities {
    /// Create empty capabilities
    pub fn new() -> Self {
        Self {
            supported_formats: Vec::new(),
            supported_resolutions: Vec::new(),
            supported_frame_rates: Vec::new(),
            supported_sample_rates: Vec::new(),
            supported_channel_counts: Vec::new(),
            buffer_size_range: None,
            capability_flags: Vec::new(),
        }
    }

    /// Check if a specific format is supported
    pub fn supports_format(&self, format: &str) -> bool {
        self.supported_formats.contains(&format.to_string())
    }

    /// Check if a specific resolution is supported (video only)
    pub fn supports_resolution(&self, width: u32, height: u32) -> bool {
        self.supported_resolutions.contains(&(width, height))
    }

    /// Check if a specific sample rate is supported (audio only)
    pub fn supports_sample_rate(&self, sample_rate: u32) -> bool {
        self.supported_sample_rates.contains(&sample_rate)
    }
}

/// Complete device information with capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDeviceInfo {
    /// Basic device information
    pub info: DeviceInfo,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
}

impl FullDeviceInfo {
    /// Create new full device info
    pub fn new(info: DeviceInfo, capabilities: DeviceCapabilities) -> Self {
        Self { info, capabilities }
    }
}

/// Device enumeration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEnumerationResult {
    /// List of discovered devices
    pub devices: Vec<FullDeviceInfo>,
    /// Enumeration timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Any errors or warnings that occurred
    pub messages: Vec<String>,
}

impl DeviceEnumerationResult {
    /// Create new enumeration result
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            timestamp: chrono::Utc::now(),
            messages: Vec::new(),
        }
    }

    /// Add a device to the result
    pub fn add_device(&mut self, device: FullDeviceInfo) {
        self.devices.push(device);
    }

    /// Add a message to the result
    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);
    }

    /// Get devices by type
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<&FullDeviceInfo> {
        self.devices
            .iter()
            .filter(|d| d.info.device_type == device_type)
            .collect()
    }

    /// Get virtual devices only
    pub fn get_virtual_devices(&self) -> Vec<&FullDeviceInfo> {
        self.devices
            .iter()
            .filter(|d| d.info.is_virtual())
            .collect()
    }

    /// Get physical devices only
    pub fn get_physical_devices(&self) -> Vec<&FullDeviceInfo> {
        self.devices
            .iter()
            .filter(|d| d.info.is_physical())
            .collect()
    }

    /// Get available devices only
    pub fn get_available_devices(&self) -> Vec<&FullDeviceInfo> {
        self.devices
            .iter()
            .filter(|d| d.info.is_available)
            .collect()
    }
}

/// Main device enumerator that coordinates all device discovery
pub struct DeviceEnumerator {
    audio_enumerator: AudioDeviceEnumerator,
    video_enumerator: VideoDeviceEnumerator,
    capability_detector: DeviceCapabilityDetector,
}

impl DeviceEnumerator {
    /// Create a new device enumerator
    pub fn new() -> Self {
        Self {
            audio_enumerator: AudioDeviceEnumerator::new(),
            video_enumerator: VideoDeviceEnumerator::new(),
            capability_detector: DeviceCapabilityDetector::new(),
        }
    }

    /// Enumerate all available devices
    pub async fn enumerate_all_devices(&self) -> Result<DeviceEnumerationResult> {
        info!("Starting comprehensive device enumeration");
        let mut result = DeviceEnumerationResult::new();

        // Enumerate audio devices
        match self.audio_enumerator.enumerate_devices().await {
            Ok(mut audio_devices) => {
                info!("Found {} audio devices", audio_devices.len());
                result.devices.append(&mut audio_devices);
            }
            Err(e) => {
                error!("Failed to enumerate audio devices: {}", e);
                result.add_message(format!("Audio enumeration error: {}", e));
            }
        }

        // Enumerate video devices
        match self.video_enumerator.enumerate_devices().await {
            Ok(mut video_devices) => {
                info!("Found {} video devices", video_devices.len());
                result.devices.append(&mut video_devices);
            }
            Err(e) => {
                error!("Failed to enumerate video devices: {}", e);
                result.add_message(format!("Video enumeration error: {}", e));
            }
        }

        info!(
            "Device enumeration completed. Total devices: {}",
            result.devices.len()
        );
        Ok(result)
    }

    /// Enumerate audio devices only
    pub async fn enumerate_audio_devices(&self) -> Result<DeviceEnumerationResult> {
        let devices = self.audio_enumerator.enumerate_devices().await?;
        Ok(DeviceEnumerationResult {
            devices,
            timestamp: chrono::Utc::now(),
            messages: Vec::new(),
        })
    }

    /// Enumerate video devices only
    pub async fn enumerate_video_devices(&self) -> Result<DeviceEnumerationResult> {
        let devices = self.video_enumerator.enumerate_devices().await?;
        Ok(DeviceEnumerationResult {
            devices,
            timestamp: chrono::Utc::now(),
            messages: Vec::new(),
        })
    }

    /// Get detailed capabilities for a specific device
    pub async fn get_device_capabilities(&self, device_id: &str) -> Result<DeviceCapabilities> {
        self.capability_detector
            .detect_capabilities(device_id)
            .await
    }

    /// Check if a device is virtual
    pub async fn is_virtual_device(&self, device_id: &str) -> Result<bool> {
        // Check audio devices
        if let Ok(audio_devices) = self.audio_enumerator.enumerate_devices().await {
            if audio_devices.iter().any(|d| d.info.id == device_id) {
                return Ok(audio_devices
                    .iter()
                    .find(|d| d.info.id == device_id)
                    .map(|d| d.info.is_virtual())
                    .unwrap_or(false));
            }
        }

        // Check video devices
        if let Ok(video_devices) = self.video_enumerator.enumerate_devices().await {
            if video_devices.iter().any(|d| d.info.id == device_id) {
                return Ok(video_devices
                    .iter()
                    .find(|d| d.info.id == device_id)
                    .map(|d| d.info.is_virtual())
                    .unwrap_or(false));
            }
        }

        Err(anyhow!("Device not found: {}", device_id))
    }
}

impl Default for DeviceEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Device filtering criteria
#[derive(Debug, Clone, Default)]
pub struct DeviceFilter {
    /// Filter by device type
    pub device_type: Option<DeviceType>,
    /// Filter by device category
    pub category: Option<DeviceCategory>,
    /// Filter by device origin
    pub origin: Option<DeviceOrigin>,
    /// Filter by availability
    pub available_only: bool,
    /// Filter by virtual/physical
    pub virtual_only: bool,
    pub physical_only: bool,
    /// Filter by driver name (partial match)
    pub driver_contains: Option<String>,
    /// Filter by name (partial match)
    pub name_contains: Option<String>,
}

impl DeviceFilter {
    /// Create new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a device matches the filter criteria
    pub fn matches(&self, device: &FullDeviceInfo) -> bool {
        // Filter by device type
        if let Some(ref device_type) = self.device_type {
            if device.info.device_type != *device_type {
                return false;
            }
        }

        // Filter by category
        if let Some(ref category) = self.category {
            if device.info.category != *category {
                return false;
            }
        }

        // Filter by origin
        if let Some(ref origin) = self.origin {
            if device.info.origin != *origin {
                return false;
            }
        }

        // Filter by availability
        if self.available_only && !device.info.is_available {
            return false;
        }

        // Filter by virtual/physical
        if self.virtual_only && !device.info.is_virtual() {
            return false;
        }

        if self.physical_only && !device.info.is_physical() {
            return false;
        }

        // Filter by driver name
        if let Some(ref driver_filter) = self.driver_contains {
            if !device
                .info
                .driver
                .to_lowercase()
                .contains(&driver_filter.to_lowercase())
            {
                return false;
            }
        }

        // Filter by name
        if let Some(ref name_filter) = self.name_contains {
            if !device
                .info
                .name
                .to_lowercase()
                .contains(&name_filter.to_lowercase())
            {
                return false;
            }
        }

        true
    }
}

/// Device filtering utilities
pub struct DeviceFilterer;

impl DeviceFilterer {
    /// Filter devices based on criteria
    pub fn filter_devices(
        devices: &[FullDeviceInfo],
        filter: &DeviceFilter,
    ) -> Vec<&FullDeviceInfo> {
        devices.iter().filter(|d| filter.matches(d)).collect()
    }

    /// Get virtual devices from a list
    pub fn get_virtual_devices(devices: &[FullDeviceInfo]) -> Vec<&FullDeviceInfo> {
        devices.iter().filter(|d| d.info.is_virtual()).collect()
    }

    /// Get physical devices from a list
    pub fn get_physical_devices(devices: &[FullDeviceInfo]) -> Vec<&FullDeviceInfo> {
        devices.iter().filter(|d| d.info.is_physical()).collect()
    }

    /// Get available devices from a list
    pub fn get_available_devices(devices: &[FullDeviceInfo]) -> Vec<&FullDeviceInfo> {
        devices.iter().filter(|d| d.info.is_available).collect()
    }

    /// Find device by ID
    pub fn find_device_by_id(
        devices: &[FullDeviceInfo],
        device_id: &str,
    ) -> Option<&FullDeviceInfo> {
        devices.iter().find(|d| d.info.id == device_id)
    }

    /// Find devices by name (partial match)
    pub fn find_devices_by_name(
        devices: &[FullDeviceInfo],
        name_pattern: &str,
    ) -> Vec<&FullDeviceInfo> {
        let pattern = name_pattern.to_lowercase();
        devices
            .iter()
            .filter(|d| d.info.name.to_lowercase().contains(&pattern))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo::new(
            "test-id".to_string(),
            "Test Device".to_string(),
            DeviceType::Audio,
            DeviceCategory::Input,
            DeviceOrigin::Virtual,
        );

        assert_eq!(device.id, "test-id");
        assert_eq!(device.name, "Test Device");
        assert_eq!(device.device_type, DeviceType::Audio);
        assert_eq!(device.category, DeviceCategory::Input);
        assert_eq!(device.origin, DeviceOrigin::Virtual);
        assert!(device.is_virtual());
        assert!(!device.is_physical());
    }

    #[test]
    fn test_device_filter() {
        let filter = DeviceFilter {
            device_type: Some(DeviceType::Audio),
            virtual_only: true,
            ..Default::default()
        };

        let virtual_audio = FullDeviceInfo::new(
            DeviceInfo::new(
                "audio-virt".to_string(),
                "Virtual Audio".to_string(),
                DeviceType::Audio,
                DeviceCategory::Input,
                DeviceOrigin::Virtual,
            ),
            DeviceCapabilities::new(),
        );

        let physical_video = FullDeviceInfo::new(
            DeviceInfo::new(
                "video-phys".to_string(),
                "Physical Video".to_string(),
                DeviceType::Video,
                DeviceCategory::Input,
                DeviceOrigin::Physical,
            ),
            DeviceCapabilities::new(),
        );

        assert!(filter.matches(&virtual_audio));
        assert!(!filter.matches(&physical_video));
    }

    #[test]
    fn test_device_enumeration_result() {
        let mut result = DeviceEnumerationResult::new();

        let device = FullDeviceInfo::new(
            DeviceInfo::new(
                "test".to_string(),
                "Test".to_string(),
                DeviceType::Audio,
                DeviceCategory::Input,
                DeviceOrigin::Physical,
            ),
            DeviceCapabilities::new(),
        );

        result.add_device(device.clone());
        assert_eq!(result.devices.len(), 1);

        let audio_devices = result.get_devices_by_type(DeviceType::Audio);
        assert_eq!(audio_devices.len(), 1);

        let video_devices = result.get_devices_by_type(DeviceType::Video);
        assert_eq!(video_devices.len(), 0);
    }
}
