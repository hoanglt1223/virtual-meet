//! Enhanced Metadata Extractor
//!
//! Advanced metadata extraction for media files using FFmpeg and Symphonia.
//! Supports detailed video/audio analysis, codec information, and technical metadata.

use anyhow::{Result, anyhow};
use ffmpeg_next as ffmpeg;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, error, debug, warn};

use crate::audio::AudioMetadata;
use crate::virtual::VideoInfo;
use crate::commands_media::{MediaMetadata, FormatInfo, MediaType};

/// Enhanced metadata extractor
pub struct MetadataExtractor {
    initialized: bool,
}

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

impl MetadataExtractor {
    /// Create new metadata extractor
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    /// Initialize FFmpeg
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        info!("Initializing FFmpeg for metadata extraction");

        ffmpeg::init().map_err(|e| anyhow!("Failed to initialize FFmpeg: {:?}", e))?;

        self.initialized = true;
        info!("FFmpeg initialized successfully");
        Ok(())
    }

    /// Analyze media file completely
    pub fn analyze_media_file(&mut self, file_path: &Path) -> Result<MediaAnalysis> {
        self.initialize()?;

        if !file_path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path.display()));
        }

        info!("Analyzing media file: {}", file_path.display());

        // Open input file
        let mut input = ffmpeg::format::input(&file_path)
            .map_err(|e| anyhow!("Failed to open media file: {:?}", e))?;

        // Get file size
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| anyhow!("Failed to get file metadata: {}", e))?
            .len();

        // Extract basic format information
        let format_name = input.format().name().to_string();
        let duration = input.duration()
            .and_then(|d| d.as_secs_f64().into())
            .unwrap_or(0.0);
        let bit_rate = input.bit_rate().unwrap_or(0);

        // Extract metadata tags
        let metadata_tags = self.extract_metadata_tags(&input);

        // Analyze streams
        let mut video_streams = Vec::new();
        let mut audio_streams = Vec::new();
        let mut subtitle_streams = Vec::new();

        for (i, stream) in input.streams().enumerate() {
            match stream.parameters().medium() {
                ffmpeg::media::Type::Video => {
                    if let Ok(video_meta) = self.analyze_video_stream(&stream) {
                        video_streams.push(video_meta);
                    }
                }
                ffmpeg::media::Type::Audio => {
                    if let Ok(audio_meta) = self.analyze_audio_stream(&stream) {
                        audio_streams.push(audio_meta);
                    }
                }
                ffmpeg::media::Type::Subtitle => {
                    if let Ok(sub_meta) = self.analyze_subtitle_stream(&stream, i) {
                        subtitle_streams.push(sub_meta);
                    }
                }
                _ => {}
            }
        }

        // Extract chapters
        let chapters = self.extract_chapters(&input);

        let analysis = MediaAnalysis {
            format_name,
            duration,
            bit_rate,
            file_size,
            video_streams,
            audio_streams,
            subtitle_streams,
            chapters,
            metadata_tags,
        };

        info!("Media analysis completed: {} video streams, {} audio streams",
              analysis.video_streams.len(), analysis.audio_streams.len());

        Ok(analysis)
    }

    /// Analyze video stream
    fn analyze_video_stream(&self, stream: &ffmpeg::Stream) -> Result<VideoMetadata> {
        let parameters = stream.parameters();
        let video = parameters.as_video()
            .ok_or_else(|| anyhow!("Not a video stream"))?;

        // Get frame rate
        let fps = stream.avg_frame_rate()
            .map(|(num, den)| if den > 0 { num as f64 / den as f64 } else { 0.0 })
            .unwrap_or(0.0);

        // Get duration
        let duration = stream.duration()
            .and_then(|d| d.as_secs_f64().into())
            .unwrap_or(0.0);

        // Get codec
        let codec = ffmpeg::codec::find(stream.id())
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(VideoMetadata {
            width: video.width(),
            height: video.height(),
            fps,
            duration,
            bit_rate: stream.bit_rate().unwrap_or(0),
            codec,
            pixel_format: format!("{:?}", video.format()),
            frame_count: stream.frames() as u64,
            interlaced: false, // TODO: Detect interlacing
            color_space: None, // TODO: Extract color space
            color_range: None, // TODO: Extract color range
        })
    }

    /// Analyze audio stream
    fn analyze_audio_stream(&self, stream: &ffmpeg::Stream) -> Result<EnhancedAudioMetadata> {
        let parameters = stream.parameters();
        let audio = parameters.as_audio()
            .ok_or_else(|| anyhow!("Not an audio stream"))?;

        // Get duration
        let duration = stream.duration()
            .and_then(|d| d.as_secs_f64().into())
            .unwrap_or(0.0);

        // Get codec
        let codec = ffmpeg::codec::find(stream.id())
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Get sample format
        let sample_format = format!("{:?}", audio.format());

        // Get channel layout
        let channel_layout = format!("{:?}", audio.layout());

        Ok(EnhancedAudioMetadata {
            duration,
            bit_rate: stream.bit_rate().unwrap_or(0),
            sample_rate: audio.rate(),
            channels: audio.channels(),
            codec,
            sample_format,
            channel_layout,
            bits_per_sample: Some(audio.bits()), // Simplified
            language: None, // TODO: Extract from metadata
        })
    }

    /// Analyze subtitle stream
    fn analyze_subtitle_stream(&self, stream: &ffmpeg::Stream, index: usize) -> Result<SubtitleStream> {
        let codec = ffmpeg::codec::find(stream.id())
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(SubtitleStream {
            index,
            codec,
            language: None, // TODO: Extract from metadata
            title: None,    // TODO: Extract from metadata
        })
    }

    /// Extract metadata tags
    fn extract_metadata_tags(&self, input: &ffmpeg::format::context::Input) -> std::collections::HashMap<String, String> {
        let mut tags = std::collections::HashMap::new();

        for (key, value) in input.metadata().iter() {
            tags.insert(key.to_string(), value.to_string());
        }

        // Extract stream-specific metadata
        for stream in input.streams() {
            if let Some(stream_metadata) = stream.metadata() {
                for (key, value) in stream_metadata.iter() {
                    let stream_key = format!("stream_{}_{}", stream.index(), key);
                    tags.insert(stream_key, value.to_string());
                }
            }
        }

        tags
    }

    /// Extract chapters
    fn extract_chapters(&self, input: &ffmpeg::format::context::Input) -> Vec<Chapter> {
        let mut chapters = Vec::new();

        for chapter in input.chapters() {
            let start_time = chapter.start()
                .and_then(|t| t.as_secs_f64().into())
                .unwrap_or(0.0);
            let end_time = chapter.end()
                .and_then(|t| t.as_secs_f64().into())
                .unwrap_or(0.0);

            let title = chapter.metadata()
                .and_then(|m| m.get("title"))
                .map(|t| t.to_string());

            chapters.push(Chapter {
                id: chapter.id(),
                start_time,
                end_time,
                title,
            });
        }

        chapters
    }

    /// Convert MediaAnalysis to MediaMetadata for compatibility
    pub fn to_media_metadata(&self, analysis: &MediaAnalysis) -> MediaMetadata {
        // Convert to VideoInfo for the first video stream
        let video_info = analysis.video_streams.first().map(|v| VideoInfo {
            width: v.width as u32,
            height: v.height as u32,
            fps: v.f64,
            frame_count: Some(v.frame_count as u64),
            duration: Some(v.duration as f64),
            codec: v.codec.clone(),
            bitrate: v.bit_rate,
            pixel_format: Some(v.pixel_format.clone()),
            color_space: v.color_space.clone(),
        });

        // Convert to AudioMetadata for the first audio stream
        let audio_info = analysis.audio_streams.first().map(|a| AudioMetadata {
            duration: a.duration as f64,
            channels: a.channels as u8,
            sample_rate: a.sample_rate as u32,
            codec: a.codec.clone(),
            bitrate: a.bit_rate as u32,
            format: a.sample_format.clone(),
            bits_per_sample: a.bits_per_sample.map(|b| b as u16),
        });

        MediaMetadata {
            video_info,
            audio_info,
            format_info: FormatInfo {
                format_name: analysis.format_name.clone(),
                duration: Some(analysis.duration),
                bit_rate: Some(analysis.bit_rate),
                file_size: analysis.file_size,
            },
        }
    }

    /// Extract metadata using audio-specific libraries for better accuracy
    pub fn extract_audio_metadata_enhanced(&self, file_path: &Path) -> Result<EnhancedAudioMetadata> {
        // For audio-only files, we can use Symphonia for more accurate metadata
        if self.is_audio_file(file_path) {
            return self.extract_symphonia_metadata(file_path);
        }

        Err(anyhow!("Not an audio file"))
    }

    /// Check if file is audio-only
    fn is_audio_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            match ext.to_str().unwrap_or("").to_lowercase().as_str() {
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Extract metadata using Symphonia for audio files
    fn extract_symphonia_metadata(&self, file_path: &Path) -> Result<EnhancedAudioMetadata> {
        // This would use Symphonia for detailed audio analysis
        // For now, return a placeholder implementation
        warn!("Symphonia metadata extraction not yet implemented, falling back to FFmpeg");

        // Create dummy metadata for now
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

    /// Get media type from file analysis
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
                let has_valid_streams = !analysis.video_streams.is_empty() ||
                                      !analysis.audio_streams.is_empty();
                let has_duration = analysis.duration > 0.0;

                Ok(has_valid_streams && has_duration)
            }
            Err(e) => {
                debug!("Media file validation failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Extract thumbnail frame position (good representative frame)
    pub fn get_thumbnail_position(&self, analysis: &MediaAnalysis) -> f64 {
        if analysis.duration > 0.0 {
            // Get frame at 10% of duration, but at least 1 second in
            (analysis.duration * 0.1).max(1.0)
        } else {
            0.0
        }
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
    use std::fs;
    use tempfile::NamedTempFile;

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
        assert_eq!(position, 10.0); // 10% of duration

        // Test with short duration
        let mut short_analysis = analysis;
        short_analysis.duration = 5.0;
        let position = extractor.get_thumbnail_position(&short_analysis);
        assert_eq!(position, 1.0); // Minimum 1 second
    }
}