//! Recording Configuration
//!
//! Configuration structures and presets for the combined recording pipeline,
//! including quality settings, resolution options, and format specifications.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};

/// Recording resolution presets
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VideoResolution {
    /// 640x480 (4:3)
    VGA,
    /// 854x480 (16:9)
    WVGA,
    /// 1280x720 (16:9)
    HD720p,
    /// 1920x1080 (16:9)
    HD1080p,
    /// 2560x1440 (16:9)
    QHD1440p,
    /// 3840x2160 (16:9)
    UHD4K,
    /// Custom resolution
    Custom(u32, u32),
}

impl VideoResolution {
    /// Get width and height for this resolution
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            VideoResolution::VGA => (640, 480),
            VideoResolution::WVGA => (854, 480),
            VideoResolution::HD720p => (1280, 720),
            VideoResolution::HD1080p => (1920, 1080),
            VideoResolution::QHD1440p => (2560, 1440),
            VideoResolution::UHD4K => (3840, 2160),
            VideoResolution::Custom(width, height) => (width, height),
        }
    }

    /// Get the aspect ratio as a string
    pub fn aspect_ratio(self) -> &'static str {
        match self {
            VideoResolution::VGA => "4:3",
            VideoResolution::WVGA => "16:9",
            VideoResolution::HD720p => "16:9",
            VideoResolution::HD1080p => "16:9",
            VideoResolution::QHD1440p => "16:9",
            VideoResolution::UHD4K => "16:9",
            VideoResolution::Custom(_, _) => "Custom",
        }
    }

    /// Get pixel count
    pub fn pixel_count(self) -> u64 {
        let (width, height) = self.dimensions();
        (width as u64) * (height as u64)
    }

    /// Get resolution name
    pub fn name(self) -> &'static str {
        match self {
            VideoResolution::VGA => "VGA (640x480)",
            VideoResolution::WVGA => "WVGA (854x480)",
            VideoResolution::HD720p => "HD 720p (1280x720)",
            VideoResolution::HD1080p => "HD 1080p (1920x1080)",
            VideoResolution::QHD1440p => "QHD 1440p (2560x1440)",
            VideoResolution::UHD4K => "UHD 4K (3840x2160)",
            VideoResolution::Custom(_, _) => "Custom",
        }
    }
}

/// Video quality presets
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VideoQualityPreset {
    /// Fast encoding, lower quality
    Fast,
    /// Balanced quality and performance
    Balanced,
    /// High quality encoding
    High,
    /// Maximum quality, slower encoding
    Ultra,
    /// Custom settings
    Custom,
}

impl VideoQualityPreset {
    /// Get the CRF (Constant Rate Factor) value for this preset
    pub fn crf_value(self) -> u8 {
        match self {
            VideoQualityPreset::Fast => 28,
            VideoQualityPreset::Balanced => 23,
            VideoQualityPreset::High => 18,
            VideoQualityPreset::Ultra => 13,
            VideoQualityPreset::Custom => 23, // Default to balanced
        }
    }

    /// Get the recommended bitrate in Mbps for 1080p
    pub fn recommended_bitrate_1080p(self) -> u32 {
        match self {
            VideoQualityPreset::Fast => 3_000_000,  // 3 Mbps
            VideoQualityPreset::Balanced => 5_000_000,  // 5 Mbps
            VideoQualityPreset::High => 8_000_000,  // 8 Mbps
            VideoQualityPreset::Ultra => 12_000_000, // 12 Mbps
            VideoQualityPreset::Custom => 5_000_000, // Default to balanced
        }
    }

    /// Get the recommended bitrate in Mbps for 720p
    pub fn recommended_bitrate_720p(self) -> u32 {
        match self {
            VideoQualityPreset::Fast => 1_500_000,  // 1.5 Mbps
            VideoQualityPreset::Balanced => 2_500_000,  // 2.5 Mbps
            VideoQualityPreset::High => 4_000_000,  // 4 Mbps
            VideoQualityPreset::Ultra => 6_000_000,  // 6 Mbps
            VideoQualityPreset::Custom => 2_500_000, // Default to balanced
        }
    }

    /// Get preset name
    pub fn name(self) -> &'static str {
        match self {
            VideoQualityPreset::Fast => "Fast (Lower Quality)",
            VideoQualityPreset::Balanced => "Balanced",
            VideoQualityPreset::High => "High Quality",
            VideoQualityPreset::Ultra => "Ultra Quality",
            VideoQualityPreset::Custom => "Custom",
        }
    }
}

/// Audio quality presets
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AudioQualityPreset {
    /// Low quality, smallest file size
    Low,
    /// Good quality for voice
    Voice,
    /// Standard quality for music
    Standard,
    /// High quality for music
    High,
    /// Maximum quality
    Lossless,
    /// Custom settings
    Custom,
}

impl AudioQualityPreset {
    /// Get the recommended audio bitrate in bps
    pub fn recommended_bitrate(self) -> u32 {
        match self {
            AudioQualityPreset::Low => 64_000,     // 64 kbps
            AudioQualityPreset::Voice => 96_000,    // 96 kbps
            AudioQualityPreset::Standard => 128_000, // 128 kbps
            AudioQualityPreset::High => 192_000,    // 192 kbps
            AudioQualityPreset::Lossless => 320_000, // 320 kbps (not truly lossless, but high)
            AudioQualityPreset::Custom => 128_000,  // Default to standard
        }
    }

    /// Get the recommended sample rate
    pub fn recommended_sample_rate(self) -> u32 {
        match self {
            AudioQualityPreset::Low => 22050,
            AudioQualityPreset::Voice => 44100,
            AudioQualityPreset::Standard => 44100,
            AudioQualityPreset::High => 48000,
            AudioQualityPreset::Lossless => 48000,
            AudioQualityPreset::Custom => 44100,
        }
    }

    /// Get preset name
    pub fn name(self) -> &'static str {
        match self {
            AudioQualityPreset::Low => "Low Quality (64 kbps)",
            AudioQualityPreset::Voice => "Voice Quality (96 kbps)",
            AudioQualityPreset::Standard => "Standard Quality (128 kbps)",
            AudioQualityPreset::High => "High Quality (192 kbps)",
            AudioQualityPreset::Lossless => "Maximum Quality (320 kbps)",
            AudioQualityPreset::Custom => "Custom",
        }
    }
}

/// Video codec options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VideoCodec {
    /// H.264 (AVC) - Most compatible
    H264,
    /// H.265 (HEVC) - Better compression
    H265,
    /// VP9 - Open source alternative
    VP9,
    /// AV1 - Next generation codec
    AV1,
}

impl VideoCodec {
    /// Get codec name
    pub fn name(self) -> &'static str {
        match self {
            VideoCodec::H264 => "H.264 (AVC)",
            VideoCodec::H265 => "H.265 (HEVC)",
            VideoCodec::VP9 => "VP9",
            VideoCodec::AV1 => "AV1",
        }
    }

    /// Get FFmpeg codec identifier
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            VideoCodec::H264 => "libx264",
            VideoCodec::H265 => "libx265",
            VideoCodec::VP9 => "libvpx-vp9",
            VideoCodec::AV1 => "libaom-av1",
        }
    }
}

/// Audio codec options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AudioCodec {
    /// AAC - Most compatible
    AAC,
    /// Opus - Modern, efficient
    Opus,
    /// MP3 - Legacy support
    MP3,
}

impl AudioCodec {
    /// Get codec name
    pub fn name(self) -> &'static str {
        match self {
            AudioCodec::AAC => "AAC",
            AudioCodec::Opus => "Opus",
            AudioCodec::MP3 => "MP3",
        }
    }

    /// Get FFmpeg codec identifier
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            AudioCodec::AAC => "aac",
            AudioCodec::Opus => "libopus",
            AudioCodec::MP3 => "libmp3lame",
        }
    }
}

/// Frame rate options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FrameRate {
    /// 15 frames per second
    FPS15,
    /// 24 frames per second (cinematic)
    FPS24,
    /// 30 frames per second
    FPS30,
    /// 60 frames per second
    FPS60,
    /// Custom frame rate
    Custom(f32),
}

impl FrameRate {
    /// Get frame rate as f32
    pub fn value(self) -> f32 {
        match self {
            FrameRate::FPS15 => 15.0,
            FrameRate::FPS24 => 24.0,
            FrameRate::FPS30 => 30.0,
            FrameRate::FPS60 => 60.0,
            FrameRate::Custom(fps) => fps,
        }
    }

    /// Get frame duration
    pub fn frame_duration(self) -> std::time::Duration {
        let fps = self.value();
        if fps > 0.0 {
            std::time::Duration::from_secs_f64(1.0 / fps as f64)
        } else {
            std::time::Duration::from_millis(33) // Default to ~30fps
        }
    }

    /// Get frame rate name
    pub fn name(self) -> String {
        match self {
            FrameRate::FPS15 => "15 FPS".to_string(),
            FrameRate::FPS24 => "24 FPS".to_string(),
            FrameRate::FPS30 => "30 FPS".to_string(),
            FrameRate::FPS60 => "60 FPS".to_string(),
            FrameRate::Custom(fps) => format!("{:.1} FPS", fps),
        }
    }
}

/// Complete recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Video settings
    pub video: VideoSettings,
    /// Audio settings
    pub audio: AudioSettings,
    /// Output settings
    pub output: OutputSettings,
    /// Advanced settings
    pub advanced: AdvancedSettings,
}

/// Video recording settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    /// Video resolution
    pub resolution: VideoResolution,
    /// Frame rate
    pub frame_rate: FrameRate,
    /// Video codec
    pub codec: VideoCodec,
    /// Quality preset
    pub quality_preset: VideoQualityPreset,
    /// Target bitrate (bits per second) - 0 for auto
    pub target_bitrate: u32,
    /// Constant rate factor (CRF) - 0 for preset default
    pub crf: u8,
    /// Keyframe interval in seconds
    pub keyframe_interval: u32,
    /// Buffer size in frames
    pub buffer_size: u32,
}

/// Audio recording settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Audio codec
    pub codec: AudioCodec,
    /// Quality preset
    pub quality_preset: AudioQualityPreset,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: u32,
    /// Target bitrate in bits per second - 0 for auto
    pub target_bitrate: u32,
    /// Buffer size in samples
    pub buffer_size: u32,
    /// Enable audio normalization
    pub enable_normalization: bool,
    /// Target loudness in LUFS (if normalization enabled)
    pub target_loudness: f32,
}

/// Output settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSettings {
    /// Output format
    pub format: OutputFormat,
    /// Output file pattern (can include placeholders)
    pub file_pattern: String,
    /// Output directory
    pub output_directory: Option<String>,
    /// Auto-create output directory
    pub auto_create_directory: bool,
    /// Maximum file size in bytes (0 for unlimited)
    max_file_size: u64,
    /// Auto-split recordings by size
    auto_split_by_size: bool,
    /// Auto-split recordings by duration
    auto_split_by_duration: bool,
    /// Maximum recording duration (0 for unlimited)
    max_duration: std::time::Duration,
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// MP4 container
    MP4,
    /// MKV container
    MKV,
    /// WebM container
    WebM,
}

impl OutputFormat {
    /// Get file extension
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::MP4 => "mp4",
            OutputFormat::MKV => "mkv",
            OutputFormat::WebM => "webm",
        }
    }

    /// Get format name
    pub fn name(self) -> &'static str {
        match self {
            OutputFormat::MP4 => "MP4",
            OutputFormat::MKV => "MKV",
            OutputFormat::WebM => "WebM",
        }
    }
}

/// Advanced recording settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    /// Enable hardware acceleration
    pub enable_hardware_acceleration: bool,
    /// Hardware acceleration device (e.g., "cuda", "vaapi")
    pub hardware_device: Option<String>,
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_usage: u8,
    /// Priority for recording thread
    pub thread_priority: ThreadPriority,
    /// Enable multi-threaded encoding
    pub enable_multi_threading: bool,
    /// Number of encoding threads (0 for auto)
    pub encoding_threads: u32,
    /// Enable preview window
    pub enable_preview: bool,
    /// Preview window position
    pub preview_position: Option<PreviewPosition>,
    /// Custom FFmpeg parameters
    pub custom_ffmpeg_params: HashMap<String, String>,
}

/// Thread priority levels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Realtime,
}

impl ThreadPriority {
    /// Get thread priority value for platform
    pub fn native_value(self) -> i32 {
        match self {
            ThreadPriority::Low => -2,
            ThreadPriority::Normal => 0,
            ThreadPriority::High => 1,
            ThreadPriority::Realtime => 2,
        }
    }
}

/// Preview window position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPosition {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl RecordingConfig {
    /// Create a new recording config with sensible defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config for 720p recording
    pub fn hd_720p() -> Self {
        let mut config = Self::default();
        config.video.resolution = VideoResolution::HD720p;
        config.video.target_bitrate = VideoQualityPreset::Balanced.recommended_bitrate_720p();
        config
    }

    /// Create a config for 1080p recording
    pub fn hd_1080p() -> Self {
        let mut config = Self::default();
        config.video.resolution = VideoResolution::HD1080p;
        config.video.target_bitrate = VideoQualityPreset::Balanced.recommended_bitrate_1080p();
        config
    }

    /// Create a config for fast recording
    pub fn fast_recording() -> Self {
        let mut config = Self::default();
        config.video.quality_preset = VideoQualityPreset::Fast;
        config.video.crf = VideoQualityPreset::Fast.crf_value();
        config.audio.quality_preset = AudioQualityPreset::Voice;
        config
    }

    /// Create a config for high quality recording
    pub fn high_quality() -> Self {
        let mut config = Self::default();
        config.video.resolution = VideoResolution::HD1080p;
        config.video.quality_preset = VideoQualityPreset::High;
        config.video.crf = VideoQualityPreset::High.crf_value();
        config.audio.quality_preset = AudioQualityPreset::High;
        config
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate video settings
        if self.video.frame_rate.value() <= 0.0 {
            return Err(anyhow!("Invalid frame rate: must be greater than 0"));
        }

        if self.video.target_bitrate > 100_000_000 {
            return Err(anyhow!("Video bitrate too high: maximum 100 Mbps"));
        }

        if self.video.crf > 51 {
            return Err(anyhow!("CRF value too high: maximum 51"));
        }

        // Validate audio settings
        if self.audio.sample_rate < 8000 || self.audio.sample_rate > 192000 {
            return Err(anyhow!("Invalid audio sample rate: must be between 8kHz and 192kHz"));
        }

        if self.audio.channels == 0 || self.audio.channels > 8 {
            return Err(anyhow!("Invalid audio channel count: must be between 1 and 8"));
        }

        if self.audio.target_bitrate > 1_000_000 {
            return Err(anyhow!("Audio bitrate too high: maximum 1 Mbps"));
        }

        // Validate output settings
        if let Some(ref directory) = self.output.output_directory {
            // Check if directory path is valid
            if directory.is_empty() {
                return Err(anyhow!("Output directory cannot be empty"));
            }
        }

        // Validate advanced settings
        if self.advanced.max_cpu_usage > 100 {
            return Err(anyhow!("Max CPU usage cannot exceed 100%"));
        }

        Ok(())
    }

    /// Get estimated file size for 1 minute of recording (in bytes)
    pub fn estimated_file_size_per_minute(&self) -> u64 {
        let video_bitrate = if self.video.target_bitrate > 0 {
            self.video.target_bitrate as u64
        } else {
            self.video.quality_preset.recommended_bitrate_1080p() as u64
        };

        let audio_bitrate = if self.audio.target_bitrate > 0 {
            self.audio.target_bitrate as u64
        } else {
            self.audio.quality_preset.recommended_bitrate() as u64
        };

        // Convert bits per second to bytes per minute
        ((video_bitrate + audio_bitrate) * 60) / 8
    }

    /// Get estimated recording duration for given file size (in seconds)
    pub fn estimated_duration_for_size(&self, file_size_bytes: u64) -> u64 {
        let video_bitrate = if self.video.target_bitrate > 0 {
            self.video.target_bitrate as u64
        } else {
            self.video.quality_preset.recommended_bitrate_1080p() as u64
        };

        let audio_bitrate = if self.audio.target_bitrate > 0 {
            self.audio.target_bitrate as u64
        } else {
            self.audio.quality_preset.recommended_bitrate() as u64
        };

        let total_bitrate = video_bitrate + audio_bitrate;
        if total_bitrate == 0 {
            return 0;
        }

        // Convert bytes to seconds
        (file_size_bytes * 8) / total_bitrate
    }
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            video: VideoSettings {
                resolution: VideoResolution::HD1080p,
                frame_rate: FrameRate::FPS30,
                codec: VideoCodec::H264,
                quality_preset: VideoQualityPreset::Balanced,
                target_bitrate: 0, // Auto
                crf: 0, // Use preset default
                keyframe_interval: 2, // 2 seconds
                buffer_size: 60, // 2 seconds at 30fps
            },
            audio: AudioSettings {
                codec: AudioCodec::AAC,
                quality_preset: AudioQualityPreset::Standard,
                sample_rate: 44100,
                channels: 2,
                target_bitrate: 0, // Auto
                buffer_size: 4096,
                enable_normalization: false,
                target_loudness: -16.0, // Standard loudness target
            },
            output: OutputSettings {
                format: OutputFormat::MP4,
                file_pattern: "recording_{timestamp}.mp4".to_string(),
                output_directory: None,
                auto_create_directory: true,
                max_file_size: 0,
                auto_split_by_size: false,
                auto_split_by_duration: false,
                max_duration: std::time::Duration::ZERO,
            },
            advanced: AdvancedSettings {
                enable_hardware_acceleration: false,
                hardware_device: None,
                max_cpu_usage: 80,
                thread_priority: ThreadPriority::High,
                enable_multi_threading: true,
                encoding_threads: 0, // Auto
                enable_preview: false,
                preview_position: None,
                custom_ffmpeg_params: HashMap::new(),
            },
        }
    }
}

impl Default for VideoSettings {
    fn default() -> Self {
        RecordingConfig::default().video
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        RecordingConfig::default().audio
    }
}

impl Default for OutputSettings {
    fn default() -> Self {
        RecordingConfig::default().output
    }
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        RecordingConfig::default().advanced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_resolution() {
        assert_eq!(VideoResolution::HD1080p.dimensions(), (1920, 1080));
        assert_eq!(VideoResolution::HD720p.dimensions(), (1280, 720));
        assert_eq!(VideoResolution::Custom(800, 600).dimensions(), (800, 600));

        assert_eq!(VideoResolution::HD1080p.aspect_ratio(), "16:9");
        assert_eq!(VideoResolution::VGA.aspect_ratio(), "4:3");

        assert_eq!(VideoResolution::HD1080p.pixel_count(), 2073600);
    }

    #[test]
    fn test_video_quality_presets() {
        assert_eq!(VideoQualityPreset::Fast.crf_value(), 28);
        assert_eq!(VideoQualityPreset::High.crf_value(), 18);

        assert_eq!(VideoQualityPreset::Balanced.recommended_bitrate_1080p(), 5_000_000);
        assert_eq!(VideoQualityPreset::High.recommended_bitrate_720p(), 4_000_000);
    }

    #[test]
    fn test_audio_quality_presets() {
        assert_eq!(AudioQualityPreset::Standard.recommended_bitrate(), 128_000);
        assert_eq!(AudioQualityPreset::High.recommended_sample_rate(), 48000);
        assert_eq!(AudioQualityPreset::Low.recommended_sample_rate(), 22050);
    }

    #[test]
    fn test_frame_rate() {
        assert_eq!(FrameRate::FPS30.value(), 30.0);
        assert_eq!(FrameRate::Custom(45.0).value(), 45.0);

        assert_eq!(FrameRate::FPS60.frame_duration().as_millis(), 16);
        assert_eq!(FrameRate::FPS30.frame_duration().as_millis(), 33);
    }

    #[test]
    fn test_codecs() {
        assert_eq!(VideoCodec::H264.ffmpeg_name(), "libx264");
        assert_eq!(VideoCodec::AV1.ffmpeg_name(), "libaom-av1");

        assert_eq!(AudioCodec::AAC.ffmpeg_name(), "aac");
        assert_eq!(AudioCodec::Opus.ffmpeg_name(), "libopus");
    }

    #[test]
    fn test_output_format() {
        assert_eq!(OutputFormat::MP4.extension(), "mp4");
        assert_eq!(OutputFormat::WebM.extension(), "webm");
    }

    #[test]
    fn test_recording_config_validation() {
        let config = RecordingConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = RecordingConfig::default();
        invalid_config.video.frame_rate = FrameRate::Custom(0.0);
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_config_presets() {
        let hd_720p = RecordingConfig::hd_720p();
        assert_eq!(hd_720p.video.resolution, VideoResolution::HD720p);

        let hd_1080p = RecordingConfig::hd_1080p();
        assert_eq!(hd_1080p.video.resolution, VideoResolution::HD1080p);

        let fast = RecordingConfig::fast_recording();
        assert_eq!(fast.video.quality_preset, VideoQualityPreset::Fast);

        let high_quality = RecordingConfig::high_quality();
        assert_eq!(high_quality.audio.quality_preset, AudioQualityPreset::High);
    }

    #[test]
    fn test_file_size_estimation() {
        let config = RecordingConfig::hd_1080p();
        let size_per_minute = config.estimated_file_size_per_minute();
        assert!(size_per_minute > 0);

        let duration_for_1gb = config.estimated_duration_for_size(1_000_000_000);
        assert!(duration_for_1gb > 0);
    }
}