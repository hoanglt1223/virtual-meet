//! Video Device Enumeration
//!
//! This module provides comprehensive video device discovery and enumeration
//! capabilities using DirectShow and Media Foundation on Windows, with the ability
//! to distinguish between physical and virtual video devices.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use windows::core::{Interface, GUID, HSTRING};
use windows::Win32::Media::DirectShow::{
    CLSID_LegacyAmFilterCategory, CLSID_SystemDeviceEnum, CLSID_VideoInputDeviceCategory,
    IBaseFilter, ICaptureGraphBuilder2, ICreateDevEnum, IEnumMoniker, IGraphBuilder, IMediaControl,
    IMoniker,
};
use windows::Win32::Media::MediaFoundation::{
    IMFActivate, IMFAttributes, MFCreateAttributes, MFEnumDeviceSources,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::PropertiesSystem::IPropertyStore;
use windows_capture::{
    capture::{CaptureControl, GraphicsCaptureApi},
    frame::Frame,
    monitor::Monitor,
    settings::{ColorFormat, Settings},
    window::Window,
};

use crate::devices::{
    DeviceCapabilities, DeviceCategory, DeviceInfo, DeviceOrigin, DeviceType, FullDeviceInfo,
};

/// Known virtual video device identifiers and patterns
const VIRTUAL_VIDEO_PATTERNS: &[&str] = &[
    "virtual",
    "virtual camera",
    "virtual webcam",
    "obs virtual",
    "obs camera",
    "splitcam",
    "manyCam",
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

/// Video device enumerator using DirectShow and Media Foundation
pub struct VideoDeviceEnumerator {
    virtual_patterns: Vec<String>,
    physical_manufacturers: Vec<String>,
}

impl VideoDeviceEnumerator {
    /// Create a new video device enumerator
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

    /// Enumerate all video devices
    pub async fn enumerate_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        info!("Enumerating video devices using DirectShow and Media Foundation");
        let mut devices = Vec::new();

        // Initialize COM for video APIs
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        // Enumerate using DirectShow
        if let Ok(dshow_devices) = self.enumerate_directshow_devices().await {
            info!("Found {} DirectShow video devices", dshow_devices.len());
            devices.extend(dshow_devices);
        } else {
            warn!("Failed to enumerate DirectShow video devices");
        }

        // Enumerate using Media Foundation
        if let Ok(mf_devices) = self.enumerate_media_foundation_devices().await {
            info!("Found {} Media Foundation video devices", mf_devices.len());
            // Merge with DirectShow results, avoiding duplicates
            for mf_device in mf_devices {
                if !devices.iter().any(|d| d.info.id == mf_device.info.id) {
                    devices.push(mf_device);
                }
            }
        } else {
            warn!("Failed to enumerate Media Foundation video devices");
        }

        // Enumerate virtual camera drivers
        if let Ok(virtual_devices) = self.enumerate_virtual_camera_drivers().await {
            info!("Found {} virtual camera drivers", virtual_devices.len());
            for virtual_device in virtual_devices {
                if !devices.iter().any(|d| d.info.id == virtual_device.info.id) {
                    devices.push(virtual_device);
                }
            }
        } else {
            warn!("Failed to enumerate virtual camera drivers");
        }

        Ok(devices)
    }

    /// Enumerate video devices using DirectShow
    async fn enumerate_directshow_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        unsafe {
            let create_dev_enum: ICreateDevEnum =
                CoCreateInstance(&CLSID_SystemDeviceEnum, None, CLSCTX_ALL)?;
            let enum_moniker: IEnumMoniker =
                create_dev_enum.CreateClassEnumerator(&CLSID_VideoInputDeviceCategory)?;

            let mut moniker_count = 0;
            let mut monikers = Vec::new();

            // Get all device monikers
            loop {
                let mut moniker: Option<IMoniker> = None;
                let mut fetched = 0u32;
                enum_moniker.Next(1, &mut moniker, Some(&mut fetched))?;

                if fetched == 0 {
                    break;
                }

                if let Some(m) = moniker {
                    monikers.push(m);
                    moniker_count += 1;
                }
            }

            info!("DirectShow found {} video device monikers", moniker_count);

            // Process each moniker
            for moniker in monikers {
                if let Ok(device_info) = self.process_directshow_moniker(&moniker).await {
                    devices.push(device_info);
                }
            }
        }

        Ok(devices)
    }

    /// Process a DirectShow device moniker
    async fn process_directshow_moniker(&self, moniker: &IMoniker) -> Result<FullDeviceInfo> {
        unsafe {
            // Get device friendly name
            let property_store: IPropertyStore =
                moniker.BindToStorage(None, &IPropertyStore::IID)?;

            let name = self
                .get_property_store_string(
                    &property_store,
                    &windows::Win32::System::PropertiesSystem::DEVPROPKEY_Device_FriendlyName,
                )
                .unwrap_or_else(|| "Unknown Video Device".to_string());

            // Get device description
            let description = self
                .get_property_store_string(
                    &property_store,
                    &windows::Win32::System::PropertiesSystem::DEVPROPKEY_Device_DeviceDesc,
                )
                .unwrap_or_else(|| "Video Capture Device".to_string());

            // Get device ID
            let device_id =
                moniker.GetDisplayName(None, &windows::Win32::System::Com::STGM_READ)?;
            let id_string = device_id.to_string();

            // Determine if device is virtual or physical
            let origin = self.determine_device_origin(&name, &id_string);

            let mut device_info = DeviceInfo::new(
                format!("dshow-{}", id_string),
                name.clone(),
                DeviceType::Video,
                DeviceCategory::Input,
                origin,
            );

            device_info.description = description;
            device_info.driver = "DirectShow".to_string();

            // Add DirectShow-specific properties
            device_info.add_property("directshow_id".to_string(), id_string);
            device_info.add_property("api".to_string(), "directshow".to_string());

            // Try to get additional device properties
            if let Ok(manufacturer) = self.get_property_store_string(
                &property_store,
                &windows::Win32::System::PropertiesSystem::DEVPROPKEY_Device_Manufacturer,
            ) {
                device_info.add_property("manufacturer".to_string(), manufacturer);
            }

            // Detect video capabilities
            let capabilities = self.detect_directshow_capabilities(moniker).await?;

            Ok(FullDeviceInfo::new(device_info, capabilities))
        }
    }

    /// Get string value from PropertyStore
    unsafe fn get_property_store_string(
        &self,
        store: &IPropertyStore,
        key: &windows::Win32::System::PropertiesSystem::PROPERTYKEY,
    ) -> Option<String> {
        if let Ok(value) = store.GetValue(key) {
            Some(value.to_string())
        } else {
            None
        }
    }

    /// Detect DirectShow device capabilities
    async fn detect_directshow_capabilities(
        &self,
        moniker: &IMoniker,
    ) -> Result<DeviceCapabilities> {
        let mut capabilities = DeviceCapabilities::new();

        unsafe {
            // Try to bind the filter to get capabilities
            if let Ok(base_filter) = moniker.BindToObject(None, &IBaseFilter::IID) {
                // This is a simplified capability detection
                // In a real implementation, you would enumerate the pins and negotiate media types

                // Add common video formats
                capabilities.supported_formats.extend_from_slice(&[
                    "YUY2".to_string(),
                    "UYVY".to_string(),
                    "RGB24".to_string(),
                    "RGB32".to_string(),
                    "MJPG".to_string(),
                    "NV12".to_string(),
                ]);

                // Add common resolutions
                capabilities.supported_resolutions.extend_from_slice(&[
                    (320, 240),   // QVGA
                    (640, 480),   // VGA
                    (800, 600),   // SVGA
                    (1024, 768),  // XGA
                    (1280, 720),  // HD 720p
                    (1920, 1080), // Full HD
                    (3840, 2160), // 4K
                ]);

                // Add common frame rates
                capabilities
                    .supported_frame_rates
                    .extend_from_slice(&[15.0, 24.0, 25.0, 30.0, 60.0, 120.0]);

                capabilities.capability_flags.push("directshow".to_string());
                capabilities
                    .capability_flags
                    .push("video_capture".to_string());
            }
        }

        // Sort for consistent output
        capabilities.supported_formats.sort_unstable();
        capabilities.supported_resolutions.sort_unstable();
        capabilities.supported_frame_rates.sort_unstable();

        Ok(capabilities)
    }

    /// Enumerate video devices using Media Foundation
    async fn enumerate_media_foundation_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        unsafe {
            let attributes: IMFAttributes = MFCreateAttributes()?;
            attributes.SetGUID(
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
            )?;

            let mut activate_objects: Vec<IMFActivate> = Vec::new();
            let mut count = 0u32;

            MFEnumDeviceSources(&attributes, Some(&mut activate_objects), &mut count)?;

            info!("Media Foundation found {} video devices", count);

            for (index, activate) in activate_objects.iter().enumerate() {
                if let Ok(device_info) = self
                    .process_media_foundation_activate(activate, index)
                    .await
                {
                    devices.push(device_info);
                }
            }
        }

        Ok(devices)
    }

    /// Process a Media Foundation device activate object
    async fn process_media_foundation_activate(
        &self,
        activate: &IMFActivate,
        index: usize,
    ) -> Result<FullDeviceInfo> {
        unsafe {
            // Get device friendly name
            let mut name_length = 0u32;
            activate.GetAllocatedString(
                &windows::Win32::Media::MediaFoundation::MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                Some(&mut name_length),
                Some(PWSTR::null()),
            )?;

            let mut name_wchar = vec![0u16; name_length as usize];
            let name_pwstr = PWSTR(name_wchar.as_mut_ptr());
            activate.GetAllocatedString(
                &windows::Win32::Media::MediaFoundation::MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                Some(&mut name_length),
                Some(name_pwstr),
            )?;

            let name = String::from_utf16_lossy(&name_wchar[..name_length as usize - 1]);

            // Get device symbol link
            let mut link_length = 0u32;
            activate.GetAllocatedString(
                &windows::Win32::Media::MediaFoundation::MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                Some(&mut link_length),
                Some(PWSTR::null()),
            )?;

            let mut link_wchar = vec![0u16; link_length as usize];
            let link_pwstr = PWSTR(link_wchar.as_mut_ptr());
            activate.GetAllocatedString(
                &windows::Win32::Media::MediaFoundation::MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                Some(&mut link_length),
                Some(link_pwstr),
            )?;

            let device_link = String::from_utf16_lossy(&link_wchar[..link_length as usize - 1]);

            // Determine if device is virtual or physical
            let origin = self.determine_device_origin(&name, &device_link);

            let mut device_info = DeviceInfo::new(
                format!("mf-{}", index),
                name.clone(),
                DeviceType::Video,
                DeviceCategory::Input,
                origin,
            );

            device_info.description = "Media Foundation Video Capture Device".to_string();
            device_info.driver = "Media Foundation".to_string();

            // Add Media Foundation-specific properties
            device_info.add_property("symbolic_link".to_string(), device_link.clone());
            device_info.add_property("api".to_string(), "media_foundation".to_string());
            device_info.add_property("activate_index".to_string(), index.to_string());

            // Detect capabilities (similar to DirectShow)
            let mut capabilities = DeviceCapabilities::new();
            capabilities.supported_formats.extend_from_slice(&[
                "YUY2".to_string(),
                "NV12".to_string(),
                "RGB32".to_string(),
                "MJPG".to_string(),
                "H264".to_string(),
            ]);
            capabilities.supported_resolutions.extend_from_slice(&[
                (640, 480),
                (1280, 720),
                (1920, 1080),
                (3840, 2160),
            ]);
            capabilities
                .supported_frame_rates
                .extend_from_slice(&[15.0, 30.0, 60.0]);
            capabilities
                .capability_flags
                .push("media_foundation".to_string());
            capabilities
                .capability_flags
                .push("video_capture".to_string());

            Ok(FullDeviceInfo::new(device_info, capabilities))
        }
    }

    /// Enumerate virtual camera drivers and software-based cameras
    async fn enumerate_virtual_camera_drivers(&self) -> Result<Vec<FullDeviceInfo>> {
        let mut devices = Vec::new();

        // Check for known virtual camera software
        let virtual_cameras = self.detect_virtual_cameras().await;

        for (name, driver, capabilities) in virtual_cameras {
            let mut device_info = DeviceInfo::new(
                format!("virtual-{}", name.to_lowercase().replace(' ', "_")),
                name.clone(),
                DeviceType::Video,
                DeviceCategory::Input,
                DeviceOrigin::Virtual,
            );

            device_info.description = format!("Virtual Camera - {}", driver);
            device_info.driver = driver;
            device_info.add_property("virtual_camera".to_string(), "true".to_string());

            devices.push(FullDeviceInfo::new(device_info, capabilities));
        }

        Ok(devices)
    }

    /// Detect known virtual camera software installations
    async fn detect_virtual_cameras(&self) -> Vec<(String, String, DeviceCapabilities)> {
        let mut virtual_cameras = Vec::new();

        // Check for OBS Virtual Camera
        if self.is_software_installed("OBS Studio") {
            let mut capabilities = DeviceCapabilities::new();
            capabilities
                .supported_formats
                .extend_from_slice(&["RGB24".to_string(), "YUY2".to_string()]);
            capabilities.supported_resolutions.extend_from_slice(&[
                (1920, 1080),
                (1280, 720),
                (640, 480),
            ]);
            capabilities
                .supported_frame_rates
                .extend_from_slice(&[30.0, 60.0]);
            capabilities
                .capability_flags
                .push("obs_virtual_camera".to_string());
            virtual_cameras.push((
                "OBS Virtual Camera".to_string(),
                "OBS Studio".to_string(),
                capabilities,
            ));
        }

        // Check for SplitCam
        if self.is_software_installed("SplitCam") {
            let mut capabilities = DeviceCapabilities::new();
            capabilities
                .supported_formats
                .extend_from_slice(&["RGB24".to_string(), "YUY2".to_string()]);
            capabilities
                .supported_resolutions
                .extend_from_slice(&[(1920, 1080), (1280, 720)]);
            capabilities
                .supported_frame_rates
                .extend_from_slice(&[30.0]);
            capabilities.capability_flags.push("splitcam".to_string());
            virtual_cameras.push((
                "SplitCam Virtual Camera".to_string(),
                "SplitCam".to_string(),
                capabilities,
            ));
        }

        // Check for ManyCam
        if self.is_software_installed("ManyCam") {
            let mut capabilities = DeviceCapabilities::new();
            capabilities
                .supported_formats
                .extend_from_slice(&["RGB24".to_string()]);
            capabilities
                .supported_resolutions
                .extend_from_slice(&[(1920, 1080), (1280, 720)]);
            capabilities
                .supported_frame_rates
                .extend_from_slice(&[30.0]);
            capabilities.capability_flags.push("manycam".to_string());
            virtual_cameras.push((
                "ManyCam Virtual Webcam".to_string(),
                "ManyCam".to_string(),
                capabilities,
            ));
        }

        virtual_cameras
    }

    /// Check if software is installed (basic implementation)
    fn is_software_installed(&self, software_name: &str) -> bool {
        // This is a simplified check - in a real implementation,
        // you would check Windows Registry, installed programs, etc.
        // For now, return false as we can't reliably detect without proper implementation
        false
    }

    /// Determine if device is virtual or physical based on name and other characteristics
    fn determine_device_origin(&self, name: &str, device_id: &str) -> DeviceOrigin {
        let name_lower = name.to_lowercase();
        let id_lower = device_id.to_lowercase();

        // Check for virtual camera patterns
        for pattern in &self.virtual_patterns {
            if name_lower.contains(pattern) || id_lower.contains(pattern) {
                return DeviceOrigin::Virtual;
            }
        }

        // Check for known physical manufacturers
        for manufacturer in &self.physical_manufacturers {
            if name_lower.contains(manufacturer) {
                return DeviceOrigin::Physical;
            }
        }

        // Additional heuristics
        if name_lower.contains("usb video device")
            || name_lower.contains("integrated camera")
            || name_lower.contains("hd pro webcam")
            || name_lower.contains("hd webcam")
            || name_lower.contains("lifecam")
            || name_lower.contains("webcam")
            || name_lower.contains("camera") && !name_lower.contains("virtual")
        {
            return DeviceOrigin::Physical;
        }

        // Check device ID for patterns
        if id_lower.contains("usb\\vid_") || // USB Vendor ID
           id_lower.contains("pci\\ven_") || // PCI Vendor ID
           id_lower.contains("acpi\\")
        {
            // ACPI device
            return DeviceOrigin::Physical;
        }

        // If we can't determine, assume unknown
        DeviceOrigin::Unknown
    }

    /// Get default video device
    pub async fn get_default_device(&self) -> Result<Option<FullDeviceInfo>> {
        let devices = self.enumerate_devices().await?;

        // Prefer physical devices as default
        for device in &devices {
            if device.info.is_physical() && device.info.is_available {
                return Ok(Some(device.clone()));
            }
        }

        // Fall back to any available device
        for device in &devices {
            if device.info.is_available {
                return Ok(Some(device.clone()));
            }
        }

        Ok(None)
    }

    /// Refresh the device list
    pub async fn refresh_devices(&self) -> Result<Vec<FullDeviceInfo>> {
        info!("Refreshing video device list");
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
    /// Check if a device supports a specific resolution
    pub fn supports_resolution(device: &FullDeviceInfo, width: u32, height: u32) -> bool {
        device
            .capabilities
            .supported_resolutions
            .contains(&(width, height))
    }

    /// Check if a device supports a specific frame rate
    pub fn supports_frame_rate(device: &FullDeviceInfo, frame_rate: f32) -> bool {
        device
            .capabilities
            .supported_frame_rates
            .contains(&frame_rate)
    }

    /// Check if a device supports a specific format
    pub fn supports_format(device: &FullDeviceInfo, format: &str) -> bool {
        device.capabilities.supported_formats.contains(&format)
    }

    /// Get recommended configuration for a device
    pub fn get_recommended_config(device: &FullDeviceInfo) -> Option<(u32, u32, f32, String)> {
        let (width, height) = device
            .capabilities
            .supported_resolutions
            .iter()
            .find(|&&(w, h)| (w == 1920 && h == 1080) || (w == 1280 && h == 720))
            .or_else(|| device.capabilities.supported_resolutions.first())?;

        let frame_rate = device
            .capabilities
            .supported_frame_rates
            .iter()
            .find(|&&fr| fr == 30.0 || fr == 60.0)
            .or_else(|| device.capabilities.supported_frame_rates.first())?;

        let format = device
            .capabilities
            .supported_formats
            .iter()
            .find(|f| f == "YUY2" || f == "RGB24" || f == "NV12")
            .or_else(|| device.capabilities.supported_formats.first())?
            .clone();

        Some((*width, *height, *frame_rate, format))
    }

    /// Get all virtual cameras from device list
    pub fn get_virtual_cameras(devices: &[FullDeviceInfo]) -> Vec<&FullDeviceInfo> {
        devices
            .iter()
            .filter(|d| d.info.device_type == DeviceType::Video && d.info.is_virtual())
            .collect()
    }

    /// Get all physical cameras from device list
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

        // Test virtual device detection
        assert_eq!(
            enumerator.determine_device_origin("OBS Virtual Camera", "test-id"),
            DeviceOrigin::Virtual
        );

        assert_eq!(
            enumerator.determine_device_origin("ManyCam Virtual Webcam", "test-id"),
            DeviceOrigin::Virtual
        );

        // Test physical device detection
        assert_eq!(
            enumerator
                .determine_device_origin("Logitech HD Pro Webcam C920", "usb\\vid_046d&pid_082d"),
            DeviceOrigin::Physical
        );

        assert_eq!(
            enumerator.determine_device_origin("Microsoft LifeCam", "usb\\vid_045e&pid_075d"),
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

        let recommended = VideoDeviceUtils::get_recommended_config(&device).unwrap();
        assert_eq!(recommended.0, 1920); // width
        assert_eq!(recommended.1, 1080); // height
        assert_eq!(recommended.2, 30.0); // frame rate
    }
}
