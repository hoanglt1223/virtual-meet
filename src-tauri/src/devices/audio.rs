//! Audio Device Enumeration
//!
//! Enumerates audio devices using CPAL, distinguishing between physical and
//! virtual devices by name patterns.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host};
use tracing::{info, warn};

use crate::devices::{
    DeviceCapabilities, DeviceCategory, DeviceInfo, DeviceOrigin, DeviceType, FullDeviceInfo,
};

/// Known virtual audio device name patterns
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
    "blackhole",
    "soundflower",
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

/// Audio device enumerator using CPAL
pub struct AudioDeviceEnumerator {
    host: Host,
    virtual_patterns: Vec<String>,
    physical_manufacturers: Vec<String>,
}

impl AudioDeviceEnumerator {
    pub fn new() -> Self {
        let host = cpal::default_host();
        Self {
            host,
            virtual_patterns: VIRTUAL_AUDIO_PATTERNS
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
            physical_manufacturers: PHYSICAL_AUDIO_MANUFACTURERS
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
        }
    }

    /// Enumerate all audio input and output devices
    pub async fn enumerate_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        info!("Enumerating audio devices via CPAL");
        let mut devices = Vec::new();

        // Input devices
        match self.host.input_devices() {
            Ok(inputs) => {
                for device in inputs {
                    if let Ok(info) = self.convert_cpal_device(&device, DeviceCategory::Input).await {
                        devices.push(info);
                    }
                }
            }
            Err(e) => warn!("Failed to enumerate audio input devices: {}", e),
        }

        // Output devices
        match self.host.output_devices() {
            Ok(outputs) => {
                for device in outputs {
                    if let Ok(info) = self.convert_cpal_device(&device, DeviceCategory::Output).await {
                        devices.push(info);
                    }
                }
            }
            Err(e) => warn!("Failed to enumerate audio output devices: {}", e),
        }

        info!("Found {} audio devices", devices.len());
        Ok(devices)
    }

    /// Convert a CPAL device to FullDeviceInfo
    async fn convert_cpal_device(
        &self,
        device: &Device,
        category: DeviceCategory,
    ) -> Result<FullDeviceInfo> {
        let name = device.name().unwrap_or_else(|_| "Unknown Audio Device".to_string());
        let cat_str = match category {
            DeviceCategory::Input => "in",
            DeviceCategory::Output => "out",
            DeviceCategory::Both => "io",
        };
        let device_id = format!("cpal-audio-{}-{}", cat_str, name.to_lowercase().replace(' ', "_"));

        let origin = self.determine_device_origin(&name, &device_id);

        let desc_str = match category {
            DeviceCategory::Input => "input",
            DeviceCategory::Output => "output",
            DeviceCategory::Both => "input/output",
        };
        let mut device_info = DeviceInfo::new(
            device_id,
            name.clone(),
            DeviceType::Audio,
            category,
            origin,
        );
        device_info.description = format!("Audio {} device", desc_str);

        // Add config properties
        if let Ok(cfg) = device.default_input_config() {
            device_info.add_property("input_sample_rate".to_string(), cfg.sample_rate().0.to_string());
            device_info.add_property("input_channels".to_string(), cfg.channels().to_string());
        }
        if let Ok(cfg) = device.default_output_config() {
            device_info.add_property("output_sample_rate".to_string(), cfg.sample_rate().0.to_string());
            device_info.add_property("output_channels".to_string(), cfg.channels().to_string());
        }

        let capabilities = self.detect_audio_capabilities(device).await;
        Ok(FullDeviceInfo::new(device_info, capabilities))
    }

    /// Detect audio capabilities from CPAL device configs
    async fn detect_audio_capabilities(&self, device: &Device) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::new();


        if let Ok(cfgs) = device.supported_input_configs() {
            for cfg in cfgs {
                let sr = cfg.min_sample_rate().0;
                if !caps.supported_sample_rates.contains(&sr) {
                    caps.supported_sample_rates.push(sr);
                }
                let ch = cfg.channels() as u32;
                if !caps.supported_channel_counts.contains(&ch) {
                    caps.supported_channel_counts.push(ch);
                }
                let fmt = format!("{:?}", cfg.sample_format());
                if !caps.supported_formats.contains(&fmt) {
                    caps.supported_formats.push(fmt);
                }
            }
        }

        if let Ok(cfgs) = device.supported_output_configs() {
            for cfg in cfgs {
                let sr = cfg.min_sample_rate().0;
                if !caps.supported_sample_rates.contains(&sr) {
                    caps.supported_sample_rates.push(sr);
                }
                let ch = cfg.channels() as u32;
                if !caps.supported_channel_counts.contains(&ch) {
                    caps.supported_channel_counts.push(ch);
                }
                let fmt = format!("{:?}", cfg.sample_format());
                if !caps.supported_formats.contains(&fmt) {
                    caps.supported_formats.push(fmt);
                }
            }
        }

        caps.supported_sample_rates.sort_unstable();
        caps.supported_channel_counts.sort_unstable();
        caps.supported_formats.sort_unstable();
        caps.capability_flags.push("audio_capture".to_string());
        caps.capability_flags.push("audio_playback".to_string());
        caps
    }

    /// Determine if device is virtual or physical based on name
    fn determine_device_origin(&self, name: &str, _device_id: &str) -> DeviceOrigin {
        let name_lower = name.to_lowercase();

        for pattern in &self.virtual_patterns {
            if name_lower.contains(pattern) {
                return DeviceOrigin::Virtual;
            }
        }

        for manufacturer in &self.physical_manufacturers {
            if name_lower.contains(manufacturer) {
                return DeviceOrigin::Physical;
            }
        }

        if name_lower.contains("high definition audio")
            || name_lower.contains("hd audio")
            || name_lower.contains("integrated")
        {
            return DeviceOrigin::Physical;
        }

        DeviceOrigin::Physical
    }

    /// Get the default device for a given category
    pub async fn get_default_device(
        &self,
        category: DeviceCategory,
    ) -> Result<Option<FullDeviceInfo>> {
        let device = match category {
            DeviceCategory::Input => self.host.default_input_device(),
            DeviceCategory::Output => self.host.default_output_device(),
            DeviceCategory::Both => self.host.default_input_device()
                .or_else(|| self.host.default_output_device()),
        };

        if let Some(d) = device {
            Ok(Some(self.convert_cpal_device(&d, category).await?))
        } else {
            Ok(None)
        }
    }

    /// Refresh device list
    pub async fn refresh_devices(&self) -> Result<Vec<FullDeviceInfo>> {
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
    pub fn supports_sample_rate(device: &FullDeviceInfo, sample_rate: u32) -> bool {
        device.capabilities.supported_sample_rates.contains(&sample_rate)
    }

    pub fn supports_channel_count(device: &FullDeviceInfo, channels: u32) -> bool {
        device.capabilities.supported_channel_counts.contains(&channels)
    }

    pub fn supports_format(device: &FullDeviceInfo, format: &str) -> bool {
        device.capabilities.supported_formats.contains(&format.to_string())
    }

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
            .find(|f| f.as_str() == "F32" || f.as_str() == "I32")
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

    #[test]
    fn test_device_origin_determination() {
        let enumerator = AudioDeviceEnumerator::new();

        assert_eq!(
            enumerator.determine_device_origin("VB-CABLE Input", ""),
            DeviceOrigin::Virtual
        );
        assert_eq!(
            enumerator.determine_device_origin("VoiceMeeter VAIO3", ""),
            DeviceOrigin::Virtual
        );
        assert_eq!(
            enumerator.determine_device_origin("Realtek High Definition Audio", ""),
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
