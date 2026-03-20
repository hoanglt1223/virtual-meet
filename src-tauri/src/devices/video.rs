//! Video Device Enumeration
//!
//! Enumerates video devices using ffmpeg CLI (dshow) on Windows.
//! Distinguishes between physical and virtual video devices by name patterns.

use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use tracing::{info, warn};

use crate::devices::{
    DeviceCapabilities, DeviceCategory, DeviceInfo, DeviceOrigin, DeviceType, FullDeviceInfo,
};

/// Known virtual video device name patterns
const VIRTUAL_VIDEO_PATTERNS: &[&str] = &[
    "virtual",
    "virtual camera",
    "virtual webcam",
    "obs virtual",
    "obs camera",
    "splitcam",
    "manycam",
    "youcam",
    "snap camera",
    "xsplit",
    "vcam",
    "webcamoid",
    "ip camera",
    "droidcam",
    "epoccam",
    "ivcam",
    "fake webcam",
    "simulated camera",
];

/// Known physical video device manufacturers
const PHYSICAL_VIDEO_MANUFACTURERS: &[&str] = &[
    "logitech",
    "microsoft",
    "rapoo",
    "aukey",
    "anker",
    "razer",
    "corsair",
    "hp",
    "dell",
    "lenovo",
    "asus",
    "sony",
    "canon",
    "nikon",
    "jvc",
    "panasonic",
    "creative",
    "trust",
    "philips",
    "a4tech",
    "genius",
    "elgato",
    "blackmagic",
    "magewell",
];

/// Video device enumerator using ffmpeg dshow
pub struct VideoDeviceEnumerator {
    virtual_patterns: Vec<String>,
    physical_manufacturers: Vec<String>,
}

impl VideoDeviceEnumerator {
    pub fn new() -> Self {
        Self {
            virtual_patterns: VIRTUAL_VIDEO_PATTERNS
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
            physical_manufacturers: PHYSICAL_VIDEO_MANUFACTURERS
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
        }
    }

    /// Enumerate all video devices via ffmpeg -list_devices
    pub async fn enumerate_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        info!("Enumerating video devices via ffmpeg dshow");

        let output = Command::new("ffmpeg")
            .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("Failed to run ffmpeg: {}", e))?;

        // ffmpeg writes device list to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        let devices = self.parse_dshow_devices(&stderr);

        info!("Found {} video devices", devices.len());
        Ok(devices)
    }

    /// Parse ffmpeg dshow device list output
    fn parse_dshow_devices(&self, stderr: &str) -> Vec<FullDeviceInfo> {
        let mut devices = Vec::new();
        let mut in_video_section = false;
        let mut index = 0usize;

        for line in stderr.lines() {
            if line.contains("DirectShow video devices") {
                in_video_section = true;
                continue;
            }
            if line.contains("DirectShow audio devices") {
                in_video_section = false;
                continue;
            }

            if in_video_section {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line.rfind('"') {
                        if end > start {
                            let name = &line[start + 1..end];
                            if !name.is_empty() && !name.contains("Alternative name") {
                                let origin = self.determine_device_origin(name, "");
                                let mut info = DeviceInfo::new(
                                    format!("dshow-video-{}", index),
                                    name.to_string(),
                                    DeviceType::Video,
                                    DeviceCategory::Input,
                                    origin,
                                );
                                info.driver = "DirectShow".to_string();
                                info.add_property("api".to_string(), "dshow".to_string());

                                let caps = self.default_video_capabilities();
                                devices.push(FullDeviceInfo::new(info, caps));
                                index += 1;
                            }
                        }
                    }
                }
            }
        }

        devices
    }

    /// Build default video capabilities
    fn default_video_capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::new();
        caps.supported_formats = vec!["YUY2".to_string(), "RGB24".to_string(), "MJPG".to_string()];
        caps.supported_resolutions = vec![(640, 480), (1280, 720), (1920, 1080)];
        caps.supported_frame_rates = vec![15.0, 30.0, 60.0];
        caps.capability_flags.push("video_capture".to_string());
        caps
    }

    /// Determine device origin from name
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

        if name_lower.contains("integrated camera")
            || name_lower.contains("hd webcam")
            || name_lower.contains("lifecam")
            || (name_lower.contains("webcam") && !name_lower.contains("virtual"))
            || (name_lower.contains("camera") && !name_lower.contains("virtual"))
        {
            return DeviceOrigin::Physical;
        }

        DeviceOrigin::Unknown
    }

    /// Get the default video device (first physical, or first available)
    pub async fn get_default_device(&self) -> Result<Option<FullDeviceInfo>> {
        let devices = self.enumerate_devices().await?;

        for device in &devices {
            if device.info.is_physical() && device.info.is_available {
                return Ok(Some(device.clone()));
            }
        }

        Ok(devices.into_iter().next())
    }

    /// Refresh device list
    pub async fn refresh_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        self.enumerate_devices().await
    }
}

impl Default for VideoDeviceEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for video device management
pub struct VideoDeviceUtils;

impl VideoDeviceUtils {
    pub fn supports_resolution(device: &FullDeviceInfo, width: u32, height: u32) -> bool {
        device.capabilities.supported_resolutions.contains(&(width, height))
    }

    pub fn supports_frame_rate(device: &FullDeviceInfo, frame_rate: f32) -> bool {
        device.capabilities.supported_frame_rates.contains(&frame_rate)
    }

    pub fn supports_format(device: &FullDeviceInfo, format: &str) -> bool {
        device.capabilities.supported_formats.contains(&format.to_string())
    }

    pub fn get_virtual_cameras(devices: &[FullDeviceInfo]) -> Vec<&FullDeviceInfo> {
        devices
            .iter()
            .filter(|d| d.info.device_type == DeviceType::Video && d.info.is_virtual())
            .collect()
    }

    pub fn get_physical_cameras(devices: &[FullDeviceInfo]) -> Vec<&FullDeviceInfo> {
        devices
            .iter()
            .filter(|d| d.info.device_type == DeviceType::Video && d.info.is_physical())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_video_device_enumerator_creation() {
        let enumerator = VideoDeviceEnumerator::new();
        assert!(!enumerator.virtual_patterns.is_empty());
        assert!(!enumerator.physical_manufacturers.is_empty());
    }

    #[test]
    fn test_device_origin_determination() {
        let enumerator = VideoDeviceEnumerator::new();

        assert_eq!(
            enumerator.determine_device_origin("OBS Virtual Camera", ""),
            DeviceOrigin::Virtual
        );

        assert_eq!(
            enumerator.determine_device_origin("ManyCam Virtual Webcam", ""),
            DeviceOrigin::Virtual
        );

        assert_eq!(
            enumerator.determine_device_origin("Logitech HD Pro Webcam C920", ""),
            DeviceOrigin::Physical
        );
    }

    #[test]
    fn test_video_device_utils() {
        let mut capabilities = DeviceCapabilities::new();
        capabilities.supported_resolutions = vec![(1920, 1080), (1280, 720), (640, 480)];
        capabilities.supported_frame_rates = vec![30.0, 60.0];
        capabilities.supported_formats = vec!["YUY2".to_string(), "RGB24".to_string()];

        let device = FullDeviceInfo::new(
            DeviceInfo::new(
                "test".to_string(),
                "Test Camera".to_string(),
                DeviceType::Video,
                DeviceCategory::Input,
                DeviceOrigin::Physical,
            ),
            capabilities,
        );

        assert!(VideoDeviceUtils::supports_resolution(&device, 1920, 1080));
        assert!(!VideoDeviceUtils::supports_resolution(&device, 3840, 2160));
        assert!(VideoDeviceUtils::supports_frame_rate(&device, 30.0));
        assert!(!VideoDeviceUtils::supports_frame_rate(&device, 120.0));
        assert!(VideoDeviceUtils::supports_format(&device, "YUY2"));
        assert!(!VideoDeviceUtils::supports_format(&device, "H264"));
    }
}
