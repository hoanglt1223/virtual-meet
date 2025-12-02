//! Device Capability Detection
//!
//! This module provides comprehensive device capability detection for both
//! audio and video devices, including format support, performance characteristics,
//! and advanced feature detection.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::devices::{DeviceCapabilities, DeviceType};

/// Capability detection configuration
#[derive(Debug, Clone)]
pub struct CapabilityDetectionConfig {
    /// Timeout for capability detection operations
    pub timeout: Duration,
    /// Whether to perform intensive capability testing
    pub intensive_testing: bool,
    /// Maximum number of formats to test
    pub max_formats_to_test: usize,
    /// Whether to cache capability results
    pub enable_caching: bool,
}

impl Default for CapabilityDetectionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            intensive_testing: false,
            max_formats_to_test: 50,
            enable_caching: true,
        }
    }
}

/// Device capability detector
pub struct DeviceCapabilityDetector {
    config: CapabilityDetectionConfig,
    capability_cache: HashMap<String, (DeviceCapabilities, Instant)>,
    audio_format_database: AudioFormatDatabase,
    video_format_database: VideoFormatDatabase,
}

impl DeviceCapabilityDetector {
    /// Create a new device capability detector
    pub fn new() -> Self {
        Self::with_config(CapabilityDetectionConfig::default())
    }

    /// Create a new detector with custom configuration
    pub fn with_config(config: CapabilityDetectionConfig) -> Self {
        Self {
            config,
            capability_cache: HashMap::new(),
            audio_format_database: AudioFormatDatabase::new(),
            video_format_database: VideoFormatDatabase::new(),
        }
    }

    /// Detect capabilities for a device by ID
    pub async fn detect_capabilities(&mut self, device_id: &str) -> Result<DeviceCapabilities> {
        info!("Detecting capabilities for device: {}", device_id);

        // Check cache first if enabled
        if self.config.enable_caching {
            if let Some((cached_capabilities, timestamp)) = self.capability_cache.get(device_id) {
                // Cache entries are valid for 1 hour
                if timestamp.elapsed() < Duration::from_secs(3600) {
                    info!("Returning cached capabilities for device: {}", device_id);
                    return Ok(cached_capabilities.clone());
                }
            }
        }

        // Determine device type from ID
        let device_type = self.determine_device_type(device_id)?;

        let capabilities = match device_type {
            DeviceType::Audio => self.detect_audio_capabilities(device_id).await?,
            DeviceType::Video => self.detect_video_capabilities(device_id).await?,
        };

        // Cache the result if enabled
        if self.config.enable_caching {
            self.capability_cache.insert(
                device_id.to_string(),
                (capabilities.clone(), Instant::now()),
            );
        }

        Ok(capabilities)
    }

    /// Determine device type from device ID
    fn determine_device_type(&self, device_id: &str) -> Result<DeviceType> {
        let id_lower = device_id.to_lowercase();

        if id_lower.contains("audio")
            || id_lower.contains("microphone")
            || id_lower.contains("speaker")
            || id_lower.contains("wasapi")
            || id_lower.contains("cpal-")
            || id_lower.contains("vb-cable")
            || id_lower.contains("voicemeeter")
        {
            Ok(DeviceType::Audio)
        } else if id_lower.contains("video")
            || id_lower.contains("camera")
            || id_lower.contains("webcam")
            || id_lower.contains("dshow-")
            || id_lower.contains("mf-")
            || id_lower.contains("virtual-")
            || id_lower.contains("obs")
        {
            Ok(DeviceType::Video)
        } else {
            // Default to audio if we can't determine
            warn!(
                "Could not determine device type for ID: {}, defaulting to Audio",
                device_id
            );
            Ok(DeviceType::Audio)
        }
    }

    /// Detect audio device capabilities
    async fn detect_audio_capabilities(&self, device_id: &str) -> Result<DeviceCapabilities> {
        info!("Detecting audio capabilities for device: {}", device_id);
        let mut capabilities = DeviceCapabilities::new();

        // Detect supported sample rates
        capabilities.supported_sample_rates = self.detect_audio_sample_rates(device_id).await?;

        // Detect supported channel counts
        capabilities.supported_channel_counts = self.detect_audio_channel_counts(device_id).await?;

        // Detect supported sample formats
        capabilities.supported_formats = self.detect_audio_sample_formats(device_id).await?;

        // Detect buffer size range
        capabilities.buffer_size_range = self.detect_audio_buffer_size_range(device_id).await?;

        // Add audio-specific capability flags
        capabilities
            .capability_flags
            .extend(self.detect_audio_capability_flags(device_id).await?);

        info!(
            "Detected {} sample rates, {} channel counts, {} formats",
            capabilities.supported_sample_rates.len(),
            capabilities.supported_channel_counts.len(),
            capabilities.supported_formats.len()
        );

        Ok(capabilities)
    }

    /// Detect supported audio sample rates
    async fn detect_audio_sample_rates(&self, device_id: &str) -> Result<Vec<u32>> {
        let mut sample_rates = Vec::new();

        // Common sample rates to test
        let common_rates = vec![
            8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
        ];

        for rate in common_rates {
            if self.test_audio_sample_rate(device_id, rate).await {
                sample_rates.push(rate);
            }
        }

        // Sort for consistency
        sample_rates.sort_unstable();
        Ok(sample_rates)
    }

    /// Test if a specific audio sample rate is supported
    async fn test_audio_sample_rate(&self, device_id: &str, sample_rate: u32) -> bool {
        // This is a simplified implementation
        // In a real implementation, you would attempt to open the device with this sample rate
        // For now, we assume most devices support common sample rates

        match sample_rate {
            44100 | 48000 => true,    // Most devices support these
            8000 | 16000 => true,     // Communication devices often support these
            88200 | 96000 => false,   // High-end devices only (conservative default)
            176400 | 192000 => false, // Professional devices only
            _ => false,               // Unknown rates
        }
    }

    /// Detect supported audio channel counts
    async fn detect_audio_channel_counts(&self, device_id: &str) -> Result<Vec<u32>> {
        let mut channel_counts = Vec::new();

        // Common channel configurations to test
        let common_channels = vec![1, 2, 4, 6, 8];

        for channels in common_channels {
            if self.test_audio_channel_count(device_id, channels).await {
                channel_counts.push(channels);
            }
        }

        // Sort for consistency
        channel_counts.sort_unstable();
        Ok(channel_counts)
    }

    /// Test if a specific audio channel count is supported
    async fn test_audio_channel_count(&self, device_id: &str, channels: u32) -> bool {
        // This is a simplified implementation
        // In a real implementation, you would attempt to open the device with this channel count

        match channels {
            1 | 2 => true,  // Most devices support mono and stereo
            4 | 6 => false, // Surround sound (conservative default)
            8 => false,     // 7.1 surround (conservative default)
            _ => false,     // Unknown configurations
        }
    }

    /// Detect supported audio sample formats
    async fn detect_audio_sample_formats(&self, device_id: &str) -> Result<Vec<String>> {
        let mut formats = Vec::new();

        // Common audio formats to test
        let common_formats = vec![
            ("U8", "Unsigned 8-bit"),
            ("I16", "Signed 16-bit"),
            ("I24", "Signed 24-bit"),
            ("I32", "Signed 32-bit"),
            ("F32", "32-bit float"),
            ("F64", "64-bit float"),
        ];

        for (format_id, _description) in common_formats {
            if self.test_audio_sample_format(device_id, format_id).await {
                formats.push(format_id.to_string());
            }
        }

        // Sort for consistency
        formats.sort_unstable();
        Ok(formats)
    }

    /// Test if a specific audio sample format is supported
    async fn test_audio_sample_format(&self, device_id: &str, format: &str) -> bool {
        // This is a simplified implementation
        // In a real implementation, you would attempt to open the device with this format

        match format {
            "I16" | "F32" => true, // Most devices support these
            "U8" | "I32" => false, // Less common
            "I24" => false,        // Professional devices
            "F64" => false,        // Rare
            _ => false,            // Unknown formats
        }
    }

    /// Detect audio buffer size range
    async fn detect_audio_buffer_size_range(
        &self,
        device_id: &str,
    ) -> Result<Option<(usize, usize)>> {
        // Common buffer size ranges
        let common_ranges = vec![
            (64, 128),
            (128, 256),
            (256, 512),
            (512, 1024),
            (1024, 2048),
            (2048, 4096),
            (4096, 8192),
        ];

        for &(min_size, max_size) in &common_ranges {
            if self
                .test_audio_buffer_range(device_id, min_size, max_size)
                .await
            {
                return Ok(Some((min_size, max_size)));
            }
        }

        // Default buffer size range
        Ok(Some((512, 2048)))
    }

    /// Test if an audio buffer size range is supported
    async fn test_audio_buffer_range(
        &self,
        _device_id: &str,
        _min_size: usize,
        _max_size: usize,
    ) -> bool {
        // This is a simplified implementation
        // Most audio devices support a range of buffer sizes
        true
    }

    /// Detect audio-specific capability flags
    async fn detect_audio_capability_flags(&self, device_id: &str) -> Result<Vec<String>> {
        let mut flags = Vec::new();

        // Basic audio capabilities
        flags.push("audio_capture".to_string());
        flags.push("audio_playback".to_string());

        // Check for advanced features based on device ID
        let id_lower = device_id.to_lowercase();

        if id_lower.contains("virtual") {
            flags.push("virtual_device".to_string());
        }

        if id_lower.contains("low_latency") || id_lower.contains("asio") {
            flags.push("low_latency".to_string());
        }

        if id_lower.contains("professional") || id_lower.contains("studio") {
            flags.push("professional_grade".to_string());
        }

        Ok(flags)
    }

    /// Detect video device capabilities
    async fn detect_video_capabilities(&self, device_id: &str) -> Result<DeviceCapabilities> {
        info!("Detecting video capabilities for device: {}", device_id);
        let mut capabilities = DeviceCapabilities::new();

        // Detect supported resolutions
        capabilities.supported_resolutions = self.detect_video_resolutions(device_id).await?;

        // Detect supported frame rates
        capabilities.supported_frame_rates = self.detect_video_frame_rates(device_id).await?;

        // Detect supported video formats
        capabilities.supported_formats = self.detect_video_formats(device_id).await?;

        // Add video-specific capability flags
        capabilities
            .capability_flags
            .extend(self.detect_video_capability_flags(device_id).await?);

        info!(
            "Detected {} resolutions, {} frame rates, {} formats",
            capabilities.supported_resolutions.len(),
            capabilities.supported_frame_rates.len(),
            capabilities.supported_formats.len()
        );

        Ok(capabilities)
    }

    /// Detect supported video resolutions
    async fn detect_video_resolutions(&self, device_id: &str) -> Result<Vec<(u32, u32)>> {
        let mut resolutions = Vec::new();

        // Common video resolutions to test
        let common_resolutions = vec![
            (320, 240),   // QVGA
            (640, 480),   // VGA
            (800, 600),   // SVGA
            (1024, 768),  // XGA
            (1280, 720),  // HD 720p
            (1920, 1080), // Full HD 1080p
            (2560, 1440), // QHD 1440p
            (3840, 2160), // 4K UHD
        ];

        for (width, height) in common_resolutions {
            if self.test_video_resolution(device_id, width, height).await {
                resolutions.push((width, height));
            }
        }

        // Sort by resolution (width, then height)
        resolutions.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        Ok(resolutions)
    }

    /// Test if a specific video resolution is supported
    async fn test_video_resolution(&self, device_id: &str, width: u32, height: u32) -> bool {
        // This is a simplified implementation
        // In a real implementation, you would attempt to configure the device with this resolution

        // Common webcam resolutions are usually supported
        match (width, height) {
            (640, 480) => true,    // VGA - almost universal
            (1280, 720) => true,   // 720p - very common
            (1920, 1080) => false, // 1080p - common but not universal
            (320, 240) => true,    // QVGA - should be supported
            (800, 600) => false,   // SVGA - less common
            _ => false,            // Unknown resolutions
        }
    }

    /// Detect supported video frame rates
    async fn detect_video_frame_rates(&self, device_id: &str) -> Result<Vec<f32>> {
        let mut frame_rates = Vec::new();

        // Common frame rates to test
        let common_frame_rates = vec![15.0, 24.0, 25.0, 30.0, 60.0, 120.0];

        for frame_rate in common_frame_rates {
            if self.test_video_frame_rate(device_id, frame_rate).await {
                frame_rates.push(frame_rate);
            }
        }

        // Sort for consistency
        frame_rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Ok(frame_rates)
    }

    /// Test if a specific video frame rate is supported
    async fn test_video_frame_rate(&self, _device_id: &str, frame_rate: f32) -> bool {
        // This is a simplified implementation
        // In a real implementation, you would attempt to configure the device with this frame rate

        match frame_rate as u32 {
            30 => true,      // 30 FPS - very common
            15 | 25 => true, // Common frame rates
            60 => false,     // 60 FPS - high-end webcams only
            24 => false,     // 24 FPS - cinema standard
            120 => false,    // 120 FPS - very rare
            _ => false,      // Unknown frame rates
        }
    }

    /// Detect supported video formats
    async fn detect_video_formats(&self, device_id: &str) -> Result<Vec<String>> {
        let mut formats = Vec::new();

        // Common video formats to test
        let common_formats = vec![
            ("YUY2", "YUV 4:2:2 packed"),
            ("UYVY", "YUV 4:2:2 packed (reversed)"),
            ("RGB24", "24-bit RGB"),
            ("RGB32", "32-bit RGB"),
            ("MJPG", "Motion JPEG"),
            ("NV12", "YUV 4:2:0 planar"),
            ("H264", "H.264 compressed"),
            ("I420", "YUV 4:2:0 planar"),
        ];

        for (format_id, _description) in common_formats {
            if self.test_video_format(device_id, format_id).await {
                formats.push(format_id.to_string());
            }
        }

        // Sort for consistency
        formats.sort_unstable();
        Ok(formats)
    }

    /// Test if a specific video format is supported
    async fn test_video_format(&self, _device_id: &str, format: &str) -> bool {
        // This is a simplified implementation
        // In a real implementation, you would attempt to configure the device with this format

        match format {
            "YUY2" | "RGB24" => true,  // Most devices support these
            "MJPG" | "NV12" => false,  // Common but not universal
            "UYVY" | "RGB32" => false, // Less common
            "H264" => false,           // Hardware encoding (rare)
            "I420" => false,           // Planar format (less common)
            _ => false,                // Unknown formats
        }
    }

    /// Detect video-specific capability flags
    async fn detect_video_capability_flags(&self, device_id: &str) -> Result<Vec<String>> {
        let mut flags = Vec::new();

        // Basic video capabilities
        flags.push("video_capture".to_string());

        // Check for advanced features based on device ID
        let id_lower = device_id.to_lowercase();

        if id_lower.contains("virtual") {
            flags.push("virtual_device".to_string());
        }

        if id_lower.contains("hd") || id_lower.contains("1080") {
            flags.push("high_definition".to_string());
        }

        if id_lower.contains("4k") || id_lower.contains("2160") {
            flags.push("ultra_hd".to_string());
        }

        if id_lower.contains("wide") || id_lower.contains("wide_angle") {
            flags.push("wide_angle".to_string());
        }

        if id_lower.contains("auto_focus") || id_lower.contains("autofocus") {
            flags.push("auto_focus".to_string());
        }

        if id_lower.contains("usb") {
            flags.push("usb_device".to_string());
        }

        Ok(flags)
    }

    /// Clear the capability cache
    pub fn clear_cache(&mut self) {
        self.capability_cache.clear();
        info!("Capability cache cleared");
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let total_entries = self.capability_cache.len();
        let expired_entries = self
            .capability_cache
            .values()
            .filter(|(_, timestamp)| timestamp.elapsed() >= Duration::from_secs(3600))
            .count();

        CacheStats {
            total_entries,
            valid_entries: total_entries - expired_entries,
            expired_entries,
        }
    }
}

impl Default for DeviceCapabilityDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of cache entries
    pub total_entries: usize,
    /// Number of valid (non-expired) entries
    pub valid_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
}

/// Audio format database with known format information
pub struct AudioFormatDatabase {
    formats: HashMap<String, AudioFormatInfo>,
}

impl AudioFormatDatabase {
    /// Create a new audio format database
    pub fn new() -> Self {
        let mut formats = HashMap::new();

        // Common audio formats
        formats.insert(
            "U8".to_string(),
            AudioFormatInfo {
                id: "U8".to_string(),
                name: "Unsigned 8-bit PCM".to_string(),
                bits_per_sample: 8,
                is_float: false,
                is_common: true,
            },
        );

        formats.insert(
            "I16".to_string(),
            AudioFormatInfo {
                id: "I16".to_string(),
                name: "Signed 16-bit PCM".to_string(),
                bits_per_sample: 16,
                is_float: false,
                is_common: true,
            },
        );

        formats.insert(
            "I24".to_string(),
            AudioFormatInfo {
                id: "I24".to_string(),
                name: "Signed 24-bit PCM".to_string(),
                bits_per_sample: 24,
                is_float: false,
                is_common: false,
            },
        );

        formats.insert(
            "I32".to_string(),
            AudioFormatInfo {
                id: "I32".to_string(),
                name: "Signed 32-bit PCM".to_string(),
                bits_per_sample: 32,
                is_float: false,
                is_common: true,
            },
        );

        formats.insert(
            "F32".to_string(),
            AudioFormatInfo {
                id: "F32".to_string(),
                name: "32-bit Float".to_string(),
                bits_per_sample: 32,
                is_float: true,
                is_common: true,
            },
        );

        formats.insert(
            "F64".to_string(),
            AudioFormatInfo {
                id: "F64".to_string(),
                name: "64-bit Double".to_string(),
                bits_per_sample: 64,
                is_float: true,
                is_common: false,
            },
        );

        Self { formats }
    }

    /// Get information about an audio format
    pub fn get_format(&self, format_id: &str) -> Option<&AudioFormatInfo> {
        self.formats.get(format_id)
    }

    /// Get all common formats
    pub fn get_common_formats(&self) -> Vec<&AudioFormatInfo> {
        self.formats.values().filter(|f| f.is_common).collect()
    }
}

/// Audio format information
#[derive(Debug, Clone)]
pub struct AudioFormatInfo {
    /// Format identifier
    pub id: String,
    /// Human-readable format name
    pub name: String,
    /// Bits per sample
    pub bits_per_sample: u8,
    /// Whether this is a floating-point format
    pub is_float: bool,
    /// Whether this is a commonly supported format
    pub is_common: bool,
}

/// Video format database with known format information
pub struct VideoFormatDatabase {
    formats: HashMap<String, VideoFormatInfo>,
}

impl VideoFormatDatabase {
    /// Create a new video format database
    pub fn new() -> Self {
        let mut formats = HashMap::new();

        // Common video formats
        formats.insert(
            "RGB24".to_string(),
            VideoFormatInfo {
                id: "RGB24".to_string(),
                name: "24-bit RGB".to_string(),
                bits_per_pixel: 24,
                compression_type: VideoCompressionType::Uncompressed,
                is_common: true,
            },
        );

        formats.insert(
            "RGB32".to_string(),
            VideoFormatInfo {
                id: "RGB32".to_string(),
                name: "32-bit RGB".to_string(),
                bits_per_pixel: 32,
                compression_type: VideoCompressionType::Uncompressed,
                is_common: true,
            },
        );

        formats.insert(
            "YUY2".to_string(),
            VideoFormatInfo {
                id: "YUY2".to_string(),
                name: "YUV 4:2:2 Packed".to_string(),
                bits_per_pixel: 16,
                compression_type: VideoCompressionType::Uncompressed,
                is_common: true,
            },
        );

        formats.insert(
            "UYVY".to_string(),
            VideoFormatInfo {
                id: "UYVY".to_string(),
                name: "YUV 4:2:2 Packed (Reversed)".to_string(),
                bits_per_pixel: 16,
                compression_type: VideoCompressionType::Uncompressed,
                is_common: false,
            },
        );

        formats.insert(
            "NV12".to_string(),
            VideoFormatInfo {
                id: "NV12".to_string(),
                name: "YUV 4:2:0 Planar".to_string(),
                bits_per_pixel: 12,
                compression_type: VideoCompressionType::Uncompressed,
                is_common: true,
            },
        );

        formats.insert(
            "MJPG".to_string(),
            VideoFormatInfo {
                id: "MJPG".to_string(),
                name: "Motion JPEG".to_string(),
                bits_per_pixel: 0, // Variable due to compression
                compression_type: VideoCompressionType::Lossy,
                is_common: true,
            },
        );

        formats.insert(
            "H264".to_string(),
            VideoFormatInfo {
                id: "H264".to_string(),
                name: "H.264/AVC".to_string(),
                bits_per_pixel: 0, // Variable due to compression
                compression_type: VideoCompressionType::Lossy,
                is_common: false,
            },
        );

        Self { formats }
    }

    /// Get information about a video format
    pub fn get_format(&self, format_id: &str) -> Option<&VideoFormatInfo> {
        self.formats.get(format_id)
    }

    /// Get all common formats
    pub fn get_common_formats(&self) -> Vec<&VideoFormatInfo> {
        self.formats.values().filter(|f| f.is_common).collect()
    }
}

/// Video format information
#[derive(Debug, Clone)]
pub struct VideoFormatInfo {
    /// Format identifier
    pub id: String,
    /// Human-readable format name
    pub name: String,
    /// Bits per pixel (0 for compressed formats)
    pub bits_per_pixel: u8,
    /// Compression type
    pub compression_type: VideoCompressionType,
    /// Whether this is a commonly supported format
    pub is_common: bool,
}

/// Video compression type
#[derive(Debug, Clone, PartialEq)]
pub enum VideoCompressionType {
    Uncompressed,
    Lossless,
    Lossy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_capability_detector_creation() {
        let detector = DeviceCapabilityDetector::new();
        let stats = detector.get_cache_stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_device_type_determination() {
        let detector = DeviceCapabilityDetector::new();

        assert_eq!(
            detector.determine_device_type("wasapi-audio-device-123"),
            Ok(DeviceType::Audio)
        );

        assert_eq!(
            detector.determine_device_type("dshow-video-camera-456"),
            Ok(DeviceType::Video)
        );
    }

    #[test]
    fn test_audio_format_database() {
        let db = AudioFormatDatabase::new();

        let format = db.get_format("I16");
        assert!(format.is_some());
        assert_eq!(format.unwrap().bits_per_sample, 16);

        let common_formats = db.get_common_formats();
        assert!(!common_formats.is_empty());
        assert!(common_formats.iter().any(|f| f.id == "F32"));
    }

    #[test]
    fn test_video_format_database() {
        let db = VideoFormatDatabase::new();

        let format = db.get_format("RGB24");
        assert!(format.is_some());
        assert_eq!(format.unwrap().bits_per_pixel, 24);

        let common_formats = db.get_common_formats();
        assert!(!common_formats.is_empty());
        assert!(common_formats.iter().any(|f| f.id == "YUY2"));
    }

    #[test]
    fn test_capability_detection_config() {
        let config = CapabilityDetectionConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert!(!config.intensive_testing);
        assert!(config.enable_caching);
        assert_eq!(config.max_formats_to_test, 50);
    }
}
