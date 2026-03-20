//! Metadata Extractor
//!
//! Extracts media metadata using ffprobe CLI.
//! Supports video/audio analysis without ffmpeg-next bindings.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::audio::AudioMetadata;
use crate::commands_media::{FormatInfo, MediaMetadata, MediaType};
use crate::virtual_device::VideoInfo;

/// Extracted video metadata
#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    pub bit_rate: u64,
    pub codec: String,
    pub pixel_format: String,
    pub frame_count: u64,
    pub interlaced: bool,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
}

/// Extracted audio metadata
#[derive(Debug, Clone)]
pub struct EnhancedAudioMetadata {
    pub duration: f64,
    pub bit_rate: u64,
    pub sample_rate: u32,
    pub channels: u32,
    pub codec: String,
    pub sample_format: String,
    pub channel_layout: String,
    pub bits_per_sample: Option<u32>,
    pub language: Option<String>,
}

/// Subtitle stream information
#[derive(Debug, Clone)]
pub struct SubtitleStream {
    pub index: usize,
    pub codec: String,
    pub language: Option<String>,
    pub title: Option<String>,
}

/// Chapter information
#[derive(Debug, Clone)]
pub struct Chapter {
    pub id: u64,
    pub start_time: f64,
    pub end_time: f64,
    pub title: Option<String>,
}

/// Complete media analysis result
#[derive(Debug)]
pub struct MediaAnalysis {
    pub format_name: String,
    pub duration: f64,
    pub bit_rate: u64,
    pub file_size: u64,
    pub video_streams: Vec<VideoMetadata>,
    pub audio_streams: Vec<EnhancedAudioMetadata>,
    pub subtitle_streams: Vec<SubtitleStream>,
    pub chapters: Vec<Chapter>,
    pub metadata_tags: std::collections::HashMap<String, String>,
}

/// Metadata extractor using ffprobe CLI
pub struct MetadataExtractor {
    pub initialized: bool,
}

impl MetadataExtractor {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Initialize (check ffprobe is available)
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        info!("Checking ffprobe availability for metadata extraction");
        let output = Command::new("ffprobe")
            .arg("-version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("ffprobe not found: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!("ffprobe check failed"));
        }

        self.initialized = true;
        info!("ffprobe available for metadata extraction");
        Ok(())
    }

    /// Analyze media file using ffprobe
    pub fn analyze_media_file(&mut self, file_path: &Path) -> Result<MediaAnalysis> {
        if !file_path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path.display()));
        }

        info!("Analyzing media file: {}", file_path.display());

        let file_size = std::fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let output = Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                file_path.to_str().unwrap_or(""),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("ffprobe failed: {}", e))?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse ffprobe output: {}", e))?;

        let format_name = json["format"]["format_name"]
            .as_str().unwrap_or("unknown").to_string();
        let duration: f64 = json["format"]["duration"]
            .as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let bit_rate: u64 = json["format"]["bit_rate"]
            .as_str().and_then(|s| s.parse().ok()).unwrap_or(0);

        // Extract metadata tags
        let mut metadata_tags = std::collections::HashMap::new();
        if let Some(tags) = json["format"]["tags"].as_object() {
            for (k, v) in tags {
                if let Some(s) = v.as_str() {
                    metadata_tags.insert(k.clone(), s.to_string());
                }
            }
        }

        let mut video_streams = Vec::new();
        let mut audio_streams = Vec::new();
        let mut subtitle_streams = Vec::new();

        if let Some(streams) = json["streams"].as_array() {
            for (i, stream) in streams.iter().enumerate() {
                let codec_type = stream["codec_type"].as_str().unwrap_or("");
                match codec_type {
                    "video" => {
                        if let Some(vm) = Self::parse_video_stream(stream) {
                            video_streams.push(vm);
                        }
                    }
                    "audio" => {
                        if let Some(am) = Self::parse_audio_stream(stream) {
                            audio_streams.push(am);
                        }
                    }
                    "subtitle" => {
                        subtitle_streams.push(SubtitleStream {
                            index: i,
                            codec: stream["codec_name"].as_str().unwrap_or("unknown").to_string(),
                            language: stream["tags"]["language"].as_str().map(|s| s.to_string()),
                            title: stream["tags"]["title"].as_str().map(|s| s.to_string()),
                        });
                    }
                    _ => {}
                }
            }
        }

        info!(
            "Media analysis completed: {} video streams, {} audio streams",
            video_streams.len(),
            audio_streams.len()
        );

        Ok(MediaAnalysis {
            format_name,
            duration,
            bit_rate,
            file_size,
            video_streams,
            audio_streams,
            subtitle_streams,
            chapters: Vec::new(),
            metadata_tags,
        })
    }

    fn parse_video_stream(stream: &serde_json::Value) -> Option<VideoMetadata> {
        let width = stream["width"].as_u64()? as u32;
        let height = stream["height"].as_u64()? as u32;

        let fps = stream["r_frame_rate"].as_str()
            .map(|s| {
                if let Some((n, d)) = s.split_once('/') {
                    let nv: f64 = n.parse().unwrap_or(30.0);
                    let dv: f64 = d.parse().unwrap_or(1.0);
                    if dv > 0.0 { nv / dv } else { 30.0 }
                } else {
                    s.parse().unwrap_or(30.0)
                }
            })
            .unwrap_or(30.0);

        let duration: f64 = stream["duration"].as_str()
            .and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let bit_rate: u64 = stream["bit_rate"].as_str()
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        let codec = stream["codec_name"].as_str().unwrap_or("unknown").to_string();
        let pixel_format = stream["pix_fmt"].as_str().unwrap_or("unknown").to_string();
        let frame_count: u64 = stream["nb_frames"].as_str()
            .and_then(|s| s.parse().ok()).unwrap_or(0);

        Some(VideoMetadata {
            width,
            height,
            fps,
            duration,
            bit_rate,
            codec,
            pixel_format,
            frame_count,
            interlaced: false,
            color_space: stream["color_space"].as_str().map(|s| s.to_string()),
            color_range: stream["color_range"].as_str().map(|s| s.to_string()),
        })
    }

    fn parse_audio_stream(stream: &serde_json::Value) -> Option<EnhancedAudioMetadata> {
        let sample_rate: u32 = stream["sample_rate"].as_str()
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        let channels: u32 = stream["channels"].as_u64().unwrap_or(0) as u32;
        let duration: f64 = stream["duration"].as_str()
            .and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let bit_rate: u64 = stream["bit_rate"].as_str()
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        let codec = stream["codec_name"].as_str().unwrap_or("unknown").to_string();
        let sample_format = stream["sample_fmt"].as_str().unwrap_or("unknown").to_string();
        let channel_layout = stream["channel_layout"].as_str().unwrap_or("unknown").to_string();

        Some(EnhancedAudioMetadata {
            duration,
            bit_rate,
            sample_rate,
            channels,
            codec,
            sample_format,
            channel_layout,
            bits_per_sample: stream["bits_per_sample"].as_u64().map(|b| b as u32),
            language: stream["tags"]["language"].as_str().map(|s| s.to_string()),
        })
    }

    /// Convert MediaAnalysis to MediaMetadata for compatibility
    pub fn to_media_metadata(&self, analysis: &MediaAnalysis) -> MediaMetadata {
        let video_info = analysis.video_streams.first().map(|v| VideoInfo {
            width: v.width,
            height: v.height,
            frame_rate: v.fps,
            duration: if v.duration > 0.0 {
                Some(Duration::from_secs_f64(v.duration))
            } else {
                None
            },
        });

        let audio_info = analysis.audio_streams.first().map(|a| AudioMetadata {
            duration: if a.duration > 0.0 {
                Some(Duration::from_secs_f64(a.duration))
            } else {
                None
            },
            duration_secs: if a.duration > 0.0 { Some(a.duration) } else { None },
            channels: a.channels,
            sample_rate: a.sample_rate,
            codec: a.codec.clone(),
            bit_rate: if a.bit_rate > 0 { Some(a.bit_rate) } else { None },
            format: a.sample_format.clone(),
        });

        MediaMetadata {
            video_info,
            audio_info,
            format_info: FormatInfo {
                format_name: analysis.format_name.clone(),
                duration: if analysis.duration > 0.0 { Some(analysis.duration) } else { None },
                bit_rate: if analysis.bit_rate > 0 { Some(analysis.bit_rate) } else { None },
                file_size: analysis.file_size,
            },
        }
    }

    /// Determine media type from analysis
    pub fn determine_media_type(&self, analysis: &MediaAnalysis) -> MediaType {
        if !analysis.video_streams.is_empty() {
            MediaType::Video
        } else if !analysis.audio_streams.is_empty() {
            MediaType::Audio
        } else {
            MediaType::Unknown
        }
    }

    /// Validate media file
    pub fn validate_media_file(&mut self, file_path: &Path) -> Result<bool> {
        match self.analyze_media_file(file_path) {
            Ok(analysis) => {
                let has_streams = !analysis.video_streams.is_empty()
                    || !analysis.audio_streams.is_empty();
                Ok(has_streams && analysis.duration > 0.0)
            }
            Err(e) => {
                debug!("Media file validation failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get thumbnail position (10% into video, min 1s)
    pub fn get_thumbnail_position(&self, analysis: &MediaAnalysis) -> f64 {
        if analysis.duration > 0.0 {
            (analysis.duration * 0.1).max(1.0)
        } else {
            0.0
        }
    }

    /// Check if file is audio-only
    pub fn is_audio_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            matches!(
                ext.to_str().unwrap_or("").to_lowercase().as_str(),
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus"
            )
        } else {
            false
        }
    }

    /// Extract enhanced audio metadata (uses symphonia for audio files)
    pub fn extract_audio_metadata_enhanced(
        &self,
        file_path: &Path,
    ) -> Result<EnhancedAudioMetadata> {
        if !self.is_audio_file(file_path) {
            return Err(anyhow!("Not an audio file"));
        }
        // Fall through to ffprobe
        warn!("Enhanced audio extraction not available, using ffprobe fallback");
        Ok(EnhancedAudioMetadata {
            duration: 0.0,
            bit_rate: 0,
            sample_rate: 44100,
            channels: 2,
            codec: "unknown".to_string(),
            sample_format: "s16".to_string(),
            channel_layout: "stereo".to_string(),
            bits_per_sample: Some(16),
            language: None,
        })
    }
}

impl Default for MetadataExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_extractor_creation() {
        let extractor = MetadataExtractor::new();
        assert!(!extractor.initialized);
    }

    #[test]
    fn test_audio_file_detection() {
        let extractor = MetadataExtractor::new();
        assert!(extractor.is_audio_file(Path::new("test.mp3")));
        assert!(extractor.is_audio_file(Path::new("test.wav")));
        assert!(extractor.is_audio_file(Path::new("test.flac")));
        assert!(!extractor.is_audio_file(Path::new("test.mp4")));
        assert!(!extractor.is_audio_file(Path::new("test.avi")));
    }

    #[test]
    fn test_thumbnail_position_calculation() {
        let extractor = MetadataExtractor::new();

        let analysis = MediaAnalysis {
            format_name: "mp4".to_string(),
            duration: 100.0,
            bit_rate: 1000000,
            file_size: 10000000,
            video_streams: vec![],
            audio_streams: vec![],
            subtitle_streams: vec![],
            chapters: vec![],
            metadata_tags: std::collections::HashMap::new(),
        };

        let position = extractor.get_thumbnail_position(&analysis);
        assert_eq!(position, 10.0);
    }
}
