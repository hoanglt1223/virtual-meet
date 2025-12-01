//! Audio Device Enumeration
//!
//! This module provides comprehensive audio device discovery and enumeration
//! capabilities using WASAPI on Windows, with the ability to distinguish
//! between physical and virtual audio devices.

use anyhow::{Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, StreamConfig, SupportedStreamConfigRange};
use std::collections::HashMap;
use tracing::{info, warn, debug, error};
use windows::Win32::Media::Audio::{
    IMMDeviceEnumerator, IMMDeviceCollection, IMMDevice,
    DEVICE_STATE_ACTIVE, DEVICE_STATE_DISABLED, DEVICE_STATE_NOTPRESENT,
    DEVICE_STATE_UNPLUGGED, eRender, eCapture, eAll,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::core::{Interface, HSTRING};

use crate::devices::{DeviceInfo, DeviceType, DeviceCategory, DeviceOrigin, FullDeviceInfo, DeviceCapabilities};

/// Known virtual audio device identifiers and patterns
const VIRTUAL_AUDIO_PATTERNS: &[&str] = &[
    "virtual",
    "virtual cable",
    "vb-cable",
    "voicemeeter",
    "obs virtual",
    "sndcpy",
    "audio repeater",
    "virtual input",
    "virtual output",
    "virtual microphone",
    "virtual speaker",
    "wave link",
    "blackhole", // macOS virtual audio
    "soundflower", // macOS virtual audio
    "loopback",
];

/// Known physical audio device manufacturers
const PHYSICAL_AUDIO_MANUFACTURERS: &[&str] = &[
    "realtek",
    "nvidia",
    "amd",
    "intel",
    "corsair",
    "logitech",
    "razer",
    "steelseries",
    "hyperx",
    "astro",
    "sennheiser",
    "bose",
    "jbl",
    "sony",
    "audio-technica",
    "shure",
    "yamaha",
    "focusrite",
    "presonus",
    "m-audio",
    "behringer",
];

/// Audio device enumerator using WASAPI
pub struct AudioDeviceEnumerator {
    host: Host,
    virtual_patterns: Vec<String>,
    physical_manufacturers: Vec<String>,
}

impl AudioDeviceEnumerator {
    /// Create a new audio device enumerator
    pub fn new() -> Self {
        let host = cpal::default_host();

        Self {
            host,
            virtual_patterns: VIRTUAL_AUDIO_PATTERNS.iter().map(|s| s.to_lowercase()).collect(),
            physical_manufacturers: PHYSICAL_AUDIO_MANUFACTURERS.iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    /// Enumerate all audio devices
    pub async fn enumerate_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        info!("Enumerating audio devices using WASAPI");
        let mut devices = Vec::new();

        // Enumerate input devices
        if let Ok(input_devices) = self.enumerate_input_devices().await {
            info!("Found {} audio input devices", input_devices.len());
            devices.extend(input_devices);
        } else {
            warn!("Failed to enumerate audio input devices");
        }

        // Enumerate output devices
        if let Ok(output_devices) = self.enumerate_output_devices().await {
            info!("Found {} audio output devices", output_devices.len());
            devices.extend(output_devices);
        } else {
            warn!("Failed to enumerate audio output devices");
        }

        Ok(devices)
    }

    /// Enumerate audio input devices (microphones)
    async fn enumerate_input_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        // Use CPAL for cross-platform enumeration
        if let Ok(input_devices) = self.host.input_devices() {
            for device in input_devices {
                if let Ok(full_info) = self.convert_cpal_device(&device, DeviceCategory::Input).await {
                    devices.push(full_info);
                }
            }
        }

        // Use Windows-specific WASAPI for more detailed information
        #[cfg(windows)]
        {
            if let Ok(wasapi_devices) = self.enumerate_wasapi_input_devices().await {
                for wasapi_device in wasapi_devices {
                    // Merge with CPAL information if available
                    if !devices.iter().any(|d| d.info.id == wasapi_device.info.id) {
                        devices.push(wasapi_device);
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Enumerate audio output devices (speakers, headphones)
    async fn enumerate_output_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        // Use CPAL for cross-platform enumeration
        if let Ok(output_devices) = self.host.output_devices() {
            for device in output_devices {
                if let Ok(full_info) = self.convert_cpal_device(&device, DeviceCategory::Output).await {
                    devices.push(full_info);
                }
            }
        }

        // Use Windows-specific WASAPI for more detailed information
        #[cfg(windows)]
        {
            if let Ok(wasapi_devices) = self.enumerate_wasapi_output_devices().await {
                for wasapi_device in wasapi_devices {
                    // Merge with CPAL information if available
                    if !devices.iter().any(|d| d.info.id == wasapi_device.info.id) {
                        devices.push(wasapi_device);
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Convert CPAL device to our format
    async fn convert_cpal_device(
        &self,
        device: &Device,
        category: DeviceCategory,
    ) -> Result<FullDeviceInfo> {
        let name = device.name().unwrap_or_else(|_| "Unknown Audio Device".to_string());
        let device_id = format!("cpal-{:?}", device.default_input_config().map(|c| c.sample_rate()));

        // Determine if device is virtual or physical
        let origin = self.determine_device_origin(&name, &device_id);

        // Create basic device info
        let mut device_info = DeviceInfo::new(
            device_id,
            name.clone(),
            DeviceType::Audio,
            category,
            origin,
        );

        // Add device description
        device_info.description = format!("Audio {} device", match category {
            DeviceCategory::Input => "input",
            DeviceCategory::Output => "output",
            DeviceCategory::Both => "input/output",
        });

        // Add CPAL-specific properties
        if let Ok(default_config) = device.default_input_config() {
            device_info.add_property(
                "default_input_sample_rate".to_string(),
                default_config.sample_rate().0.to_string(),
            );
            device_info.add_property(
                "default_input_channels".to_string(),
                default_config.channels().to_string(),
            );
            device_info.add_property(
                "default_input_format".to_string(),
                format!("{:?}", default_config.sample_format()),
            );
        }

        if let Ok(default_config) = device.default_output_config() {
            device_info.add_property(
                "default_output_sample_rate".to_string(),
                default_config.sample_rate().0.to_string(),
            );
            device_info.add_property(
                "default_output_channels".to_string(),
                default_config.channels().to_string(),
            );
            device_info.add_property(
                "default_output_format".to_string(),
                format!("{:?}", default_config.sample_format()),
            );
        }

        // Detect device capabilities
        let capabilities = self.detect_audio_capabilities(device).await?;

        Ok(FullDeviceInfo::new(device_info, capabilities))
    }

    /// Determine if device is virtual or physical based on name and other characteristics
    fn determine_device_origin(&self, name: &str, device_id: &str) -> DeviceOrigin {
        let name_lower = name.to_lowercase();

        // Check for virtual audio patterns
        for pattern in &self.virtual_patterns {
            if name_lower.contains(pattern) {
                return DeviceOrigin::Virtual;
            }
        }

        // Check device ID for virtual indicators
        let id_lower = device_id.to_lowercase();
        if id_lower.contains("virtual") || id_lower.contains("cable") || id_lower.contains("voicemeeter") {
            return DeviceOrigin::Virtual;
        }

        // Check for known physical manufacturers
        for manufacturer in &self.physical_manufacturers {
            if name_lower.contains(manufacturer) {
                return DeviceOrigin::Physical;
            }
        }

        // Additional heuristics
        if name_lower.contains("realtek") ||
           name_lower.contains("high definition audio") ||
           name_lower.contains("nvidia high definition audio") ||
           name_lower.contains("amd high definition audio") ||
           name_lower.contains("intel display audio") {
            return DeviceOrigin::Physical;
        }

        // If we can't determine, assume physical (more conservative)
        DeviceOrigin::Physical
    }

    /// Detect audio device capabilities
    async fn detect_audio_capabilities(&self, device: &Device) -> Result<DeviceCapabilities> {
        let mut capabilities = DeviceCapabilities::new();

        // Get supported configurations
        if let Ok(configs) = device.supported_input_configs() {
            for config in configs {
                // Add sample rate
                if !capabilities.supported_sample_rates.contains(&config.sample_rate().0) {
                    capabilities.supported_sample_rates.push(config.sample_rate().0);
                }

                // Add channel count
                if !capabilities.supported_channel_counts.contains(&config.channels()) {
                    capabilities.supported_channel_counts.push(config.channels());
                }

                // Add sample format
                let format_str = format!("{:?}", config.sample_format());
                if !capabilities.supported_formats.contains(&format_str) {
                    capabilities.supported_formats.push(format_str);
                }

                // Add buffer size range if available
                if let Some(buffer_size) = config.buffer_size() {
                    match buffer_size {
                        cpal::BufferSize::Range(min, max) => {
                            if let Some((existing_min, existing_max)) = capabilities.buffer_size_range {
                                capabilities.buffer_size_range = Some((
                                    existing_min.min(*min as usize),
                                    existing_max.max(*max as usize),
                                ));
                            } else {
                                capabilities.buffer_size_range = Some((*min as usize, *max as usize));
                            }
                        }
                        cpal::BufferSize::Fixed(size) => {
                            if let Some((existing_min, existing_max)) = capabilities.buffer_size_range {
                                capabilities.buffer_size_range = Some((
                                    existing_min.min(*size as usize),
                                    existing_max.max(*size as usize),
                                ));
                            } else {
                                capabilities.buffer_size_range = Some((*size as usize, *size as usize));
                            }
                        }
                    }
                }
            }
        }

        if let Ok(configs) = device.supported_output_configs() {
            for config in configs {
                // Add sample rate
                if !capabilities.supported_sample_rates.contains(&config.sample_rate().0) {
                    capabilities.supported_sample_rates.push(config.sample_rate().0);
                }

                // Add channel count
                if !capabilities.supported_channel_counts.contains(&config.channels()) {
                    capabilities.supported_channel_counts.push(config.channels());
                }

                // Add sample format
                let format_str = format!("{:?}", config.sample_format());
                if !capabilities.supported_formats.contains(&format_str) {
                    capabilities.supported_formats.push(format_str);
                }
            }
        }

        // Sort the arrays for consistent output
        capabilities.supported_sample_rates.sort_unstable();
        capabilities.supported_channel_counts.sort_unstable();
        capabilities.supported_formats.sort_unstable();

        // Add common capability flags
        capabilities.capability_flags.push("audio_capture".to_string());
        capabilities.capability_flags.push("audio_playback".to_string());

        Ok(capabilities)
    }

    /// Get detailed WASAPI device information (Windows only)
    #[cfg(windows)]
    async fn enumerate_wasapi_input_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(
                &windows::Win32::Media::Audio::MMDeviceEnumerator::default(),
                None,
                CLSCTX_ALL,
            )?;

            let collection: IMMDeviceCollection = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;

            let count = collection.GetCount()?;
            for i in 0..count {
                let imm_device: IMMDevice = collection.Item(i)?;

                if let Ok(device_info) = self.convert_wasapi_device(&imm_device, DeviceCategory::Input).await {
                    devices.push(device_info);
                }
            }
        }

        Ok(devices)
    }

    /// Get detailed WASAPI device information (Windows only)
    #[cfg(windows)]
    async fn enumerate_wasapi_output_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(
                &windows::Win32::Media::Audio::MMDeviceEnumerator::default(),
                None,
                CLSCTX_ALL,
            )?;

            let collection: IMMDeviceCollection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;

            let count = collection.GetCount()?;
            for i in 0..count {
                let imm_device: IMMDevice = collection.Item(i)?;

                if let Ok(device_info) = self.convert_wasapi_device(&imm_device, DeviceCategory::Output).await {
                    devices.push(device_info);
                }
            }
        }

        Ok(devices)
    }

    /// Convert WASAPI device to our format (Windows only)
    #[cfg(windows)]
    async fn convert_wasapi_device(
        &self,
        device: &IMMDevice,
        category: DeviceCategory,
    ) -> Result<FullDeviceInfo> {
        unsafe {
            let id = device.GetId()?;
            let id_string = id.to_string();

            let properties = device.OpenPropertyStore(windows::Win32::Media::Audio::STGM_READ)?;

            // Get device friendly name
            let name = properties
                .GetValue(&windows::Win32::Media::Audio::DEVPKEY_Device_FriendlyName)?
                .to_string();

            // Get device description
            let description = properties
                .GetValue(&windows::Win32::Media::Audio::DEVPKEY_Device_DeviceDesc)?
                .to_string();

            // Get device interface friendly name (often contains driver info)
            let interface_name = properties
                .GetValue(&windows::Win32::Media::Audio::DEVPKEY_DeviceInterface_FriendlyName)?
                .to_string();

            // Determine if device is virtual or physical
            let origin = self.determine_device_origin(&name, &id_string);

            let mut device_info = DeviceInfo::new(
                format!("wasapi-{}", id_string),
                name,
                DeviceType::Audio,
                category,
                origin,
            );

            device_info.description = description;
            device_info.driver = interface_name;

            // Add WASAPI-specific properties
            device_info.add_property("wasapi_id".to_string(), id_string);
            device_info.add_property("device_state".to_string(), "active".to_string());

            // Try to get additional device properties
            if let Ok(manufacturer) = properties.GetValue(&windows::Win32::Media::Audio::DEVPKEY_Device_Manufacturer) {
                device_info.add_property("manufacturer".to_string(), manufacturer.to_string());
            }

            if let Ok(device_class) = properties.GetValue(&windows::Win32::Media::Audio::DEVPKEY_Device_Class) {
                device_info.add_property("device_class".to_string(), device_class.to_string());
            }

            // Detect capabilities (basic set for WASAPI)
            let mut capabilities = DeviceCapabilities::new();
            capabilities.supported_sample_rates = vec![8000, 11025, 16000, 22050, 44100, 48000, 88200, 96000, 176400, 192000];
            capabilities.supported_channel_counts = vec![1, 2, 4, 6, 8];
            capabilities.supported_formats = vec!["I16".to_string(), "I32".to_string(), "F32".to_string()];
            capabilities.capability_flags.push("wasapi".to_string());

            Ok(FullDeviceInfo::new(device_info, capabilities))
        }
    }

    /// Get default audio device for a given category
    pub async fn get_default_device(&self, category: DeviceCategory) -> Result<Option<FullDeviceInfo>> {
        let device = match category {
            DeviceCategory::Input => self.host.default_input_device(),
            DeviceCategory::Output => self.host.default_output_device(),
            DeviceCategory::Both => {
                // For "both", try input first, then output
                if let Ok(device) = self.host.default_input_device() {
                    Some(device)
                } else {
                    self.host.default_output_device().ok()
                }
            }
        };

        if let Some(device) = device {
            Ok(Some(self.convert_cpal_device(&device, category).await?))
        } else {
            Ok(None)
        }
    }

    /// Refresh the device list
    pub async fn refresh_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        info!("Refreshing audio device list");
        self.enumerate_devices().await
    }
}

impl Default for AudioDeviceEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for audio device management
pub struct AudioDeviceUtils;

impl AudioDeviceUtils {
    /// Check if a device supports a specific sample rate
    pub fn supports_sample_rate(device: &FullDeviceInfo, sample_rate: u32) -> bool {
        device.capabilities.supported_sample_rates.contains(&sample_rate)
    }

    /// Check if a device supports a specific channel count
    pub fn supports_channel_count(device: &FullDeviceInfo, channels: u32) -> bool {
        device.capabilities.supported_channel_counts.contains(&channels)
    }

    /// Check if a device supports a specific format
    pub fn supports_format(device: &FullDeviceInfo, format: &str) -> bool {
        device.capabilities.supported_formats.contains(&format)
    }

    /// Get recommended configuration for a device
    pub fn get_recommended_config(device: &FullDeviceInfo) -> Option<(u32, u32, String)> {
        let sample_rate = device.capabilities.supported_sample_rates
            .iter()
            .find(|&&sr| sr == 44100 || sr == 48000)
            .or_else(|| device.capabilities.supported_sample_rates.first())?;

        let channels = device.capabilities.supported_channel_counts
            .iter()
            .find(|&&c| c == 2)
            .or_else(|| device.capabilities.supported_channel_counts.first())?;

        let format = device.capabilities.supported_formats
            .iter()
            .find(|f| f == "F32" || f == "I32")
            .or_else(|| device.capabilities.supported_formats.first())?
            .clone();

        Some((*sample_rate, *channels, format))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_device_enumerator_creation() {
        let enumerator = AudioDeviceEnumerator::new();
        assert!(!enumerator.virtual_patterns.is_empty());
        assert!(!enumerator.physical_manufacturers.is_empty());
    }

    #[tokio::test]
    async fn test_device_origin_determination() {
        let enumerator = AudioDeviceEnumerator::new();

        // Test virtual device detection
        assert_eq!(
            enumerator.determine_device_origin("VB-CABLE Input", "test-id"),
            DeviceOrigin::Virtual
        );

        assert_eq!(
            enumerator.determine_device_origin("VoiceMeeter VAIO3", "test-id"),
            DeviceOrigin::Virtual
        );

        // Test physical device detection
        assert_eq!(
            enumerator.determine_device_origin("Realtek High Definition Audio", "test-id"),
            DeviceOrigin::Physical
        );

        assert_eq!(
            enumerator.determine_device_origin("NVIDIA High Definition Audio", "test-id"),
            DeviceOrigin::Physical
        );
    }

    #[test]
    fn test_audio_device_utils() {
        let mut capabilities = DeviceCapabilities::new();
        capabilities.supported_sample_rates = vec![44100, 48000];
        capabilities.supported_channel_counts = vec![1, 2];
        capabilities.supported_formats = vec!["I16".to_string(), "F32".to_string()];

        let device = FullDeviceInfo::new(
            DeviceInfo::new(
                "test".to_string(),
                "Test".to_string(),
                DeviceType::Audio,
                DeviceCategory::Input,
                DeviceOrigin::Physical,
            ),
            capabilities,
        );

        assert!(AudioDeviceUtils::supports_sample_rate(&device, 44100));
        assert!(!AudioDeviceUtils::supports_sample_rate(&device, 96000));
        assert!(AudioDeviceUtils::supports_channel_count(&device, 2));
        assert!(!AudioDeviceUtils::supports_channel_count(&device, 8));
        assert!(AudioDeviceUtils::supports_format(&device, "F32"));
        assert!(!AudioDeviceUtils::supports_format(&device, "U8"));
    }
}