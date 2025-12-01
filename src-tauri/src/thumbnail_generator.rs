//! Thumbnail Generator
//!
//! Generates thumbnails for video and image files using FFmpeg and image processing.
//! Supports multiple thumbnail sizes and formats.

use anyhow::{Result, anyhow};
use ffmpeg_next as ffmpeg;
use image::{DynamicImage, ImageFormat, RgbImage, Rgb};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, error, debug, warn};
use tempfile::NamedTempFile;

use crate::metadata_extractor::{MetadataExtractor, MediaAnalysis};

/// Thumbnail configuration
#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    pub width: u32,
    pub height: u32,
    pub quality: u8, // 1-100, for JPEG quality
    pub format: ThumbnailFormat,
    pub maintain_aspect_ratio: bool,
}

/// Thumbnail format
#[derive(Debug, Clone)]
pub enum ThumbnailFormat {
    JPEG,
    PNG,
    WEBP,
}

impl ThumbnailFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ThumbnailFormat::JPEG => "jpg",
            ThumbnailFormat::PNG => "png",
            ThumbnailFormat::WEBP => "webp",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ThumbnailFormat::JPEG => "image/jpeg",
            ThumbnailFormat::PNG => "image/png",
            ThumbnailFormat::WEBP => "image/webp",
        }
    }
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            width: 320,
            height: 240,
            quality: 85,
            format: ThumbnailFormat::JPEG,
            maintain_aspect_ratio: true,
        }
    }
}

/// Thumbnail generator
pub struct ThumbnailGenerator {
    metadata_extractor: MetadataExtractor,
    config: ThumbnailConfig,
}

/// Thumbnail generation result
#[derive(Debug)]
pub struct ThumbnailResult {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub format: String,
}

impl ThumbnailGenerator {
    /// Create new thumbnail generator
    pub fn new(config: ThumbnailConfig) -> Self {
        Self {
            metadata_extractor: MetadataExtractor::new(),
            config,
        }
    }

    /// Create thumbnail generator with default config
    pub fn new_default() -> Self {
        Self::new(ThumbnailConfig::default())
    }

    /// Generate thumbnail for video file
    pub async fn generate_video_thumbnail(&mut self, video_path: &Path, output_dir: &Path) -> Result<ThumbnailResult> {
        info!("Generating video thumbnail for: {}", video_path.display());

        if !video_path.exists() {
            return Err(anyhow!("Video file does not exist: {}", video_path.display()));
        }

        // Analyze video to get metadata and thumbnail position
        let analysis = self.metadata_extractor.analyze_media_file(video_path)
            .map_err(|e| anyhow!("Failed to analyze video: {}", e))?;

        if analysis.video_streams.is_empty() {
            return Err(anyhow!("No video streams found in file"));
        }

        let video_info = &analysis.video_streams[0];
        let thumbnail_time = self.metadata_extractor.get_thumbnail_position(&analysis);

        debug!("Extracting thumbnail at {:.2} seconds", thumbnail_time);

        // Generate thumbnail using FFmpeg
        let thumbnail_path = self.generate_ffmpeg_thumbnail(
            video_path,
            output_dir,
            thumbnail_time,
            &self.config,
            video_info.width,
            video_info.height,
        ).await?;

        // Get thumbnail file size
        let file_size = tokio::fs::metadata(&thumbnail_path).await
            .map_err(|e| anyhow!("Failed to get thumbnail metadata: {}", e))?
            .len();

        Ok(ThumbnailResult {
            path: thumbnail_path,
            width: self.config.width,
            height: self.config.height,
            file_size,
            format: self.config.format.extension().to_string(),
        })
    }

    /// Generate thumbnail for image file
    pub async fn generate_image_thumbnail(&self, image_path: &Path, output_dir: &Path) -> Result<ThumbnailResult> {
        info!("Generating image thumbnail for: {}", image_path.display());

        if !image_path.exists() {
            return Err(anyhow!("Image file does not exist: {}", image_path.display()));
        }

        // Load image
        let img = tokio::task::spawn_blocking(move || {
            image::open(image_path)
        }).await
            .map_err(|e| anyhow!("Failed to load image: {}", e))?
            .map_err(|e| anyhow!("Failed to open image: {}", e))?;

        // Generate thumbnail filename
        let filename = self.generate_thumbnail_filename(image_path);
        let thumbnail_path = output_dir.join(filename);

        // Resize and save thumbnail
        let thumbnail_img = self.resize_image(&img, &self.config);

        tokio::task::spawn_blocking(move || {
            self.save_thumbnail(&thumbnail_img, &thumbnail_path, &self.config)
        }).await
            .map_err(|e| anyhow!("Failed to save thumbnail: {}", e))??;

        // Get thumbnail file size
        let file_size = tokio::fs::metadata(&thumbnail_path).await
            .map_err(|e| anyhow!("Failed to get thumbnail metadata: {}", e))?
            .len();

        Ok(ThumbnailResult {
            path: thumbnail_path,
            width: thumbnail_img.width(),
            height: thumbnail_img.height(),
            file_size,
            format: self.config.format.extension().to_string(),
        })
    }

    /// Generate thumbnail using FFmpeg (async wrapper)
    async fn generate_ffmpeg_thumbnail(
        &self,
        video_path: &Path,
        output_dir: &Path,
        seek_time: f64,
        config: &ThumbnailConfig,
        video_width: u32,
        video_height: u32,
    ) -> Result<PathBuf> {
        let video_path = video_path.to_path_buf();
        let output_dir = output_dir.to_path_buf();
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            Self::generate_ffmpeg_thumbnail_sync(
                &video_path,
                &output_dir,
                seek_time,
                &config,
                video_width,
                video_height,
            )
        }).await
            .map_err(|e| anyhow!("Thumbnail generation task failed: {}", e))?
    }

    /// Generate thumbnail using FFmpeg (sync implementation)
    fn generate_ffmpeg_thumbnail_sync(
        video_path: &Path,
        output_dir: &Path,
        seek_time: f64,
        config: &ThumbnailConfig,
        video_width: u32,
        video_height: u32,
    ) -> Result<PathBuf> {
        // Initialize FFmpeg if needed
        if !ffmpeg::is_initialized() {
            ffmpeg::init().map_err(|e| anyhow!("Failed to initialize FFmpeg: {:?}", e))?;
        }

        // Generate thumbnail filename
        let filename = Self::generate_thumbnail_filename_for_path(video_path, &config.format);
        let thumbnail_path = output_dir.join(filename);

        // Calculate target dimensions maintaining aspect ratio
        let (target_width, target_height) = if config.maintain_aspect_ratio {
            Self::calculate_dimensions(video_width, video_height, config.width, config.height)
        } else {
            (config.width, config.height)
        };

        // Use FFmpeg command line for thumbnail generation (more reliable than ffmpeg-next)
        let seek_time_str = format!("{:.3}", seek_time);
        let scale_filter = format!("scale={}:{}", target_width, target_height);

        let cmd = std::process::Command::new("ffmpeg")
            .arg("-ss")
            .arg(&seek_time_str)
            .arg("-i")
            .arg(video_path)
            .arg("-vframes")
            .arg("1")
            .arg("-vf")
            .arg(&scale_filter)
            .arg("-q:v")
            .arg(config.quality.to_string())
            .arg("-y")
            .arg(&thumbnail_path)
            .output()
            .map_err(|e| anyhow!("Failed to execute FFmpeg: {}", e))?;

        if !cmd.status.success() {
            let stderr = String::from_utf8_lossy(&cmd.stderr);
            return Err(anyhow!("FFmpeg failed: {}", stderr));
        }

        Ok(thumbnail_path)
    }

    /// Resize image maintaining aspect ratio
    fn resize_image(&self, img: &DynamicImage, config: &ThumbnailConfig) -> DynamicImage {
        let (original_width, original_height) = img.dimensions();

        let (target_width, target_height) = if config.maintain_aspect_ratio {
            Self::calculate_dimensions(original_width, original_height, config.width, config.height)
        } else {
            (config.width, config.height)
        }

        img.resize(target_width, target_height, image::imageops::FilterType::Lanczos3)
    }

    /// Calculate target dimensions maintaining aspect ratio
    fn calculate_dimensions(original_width: u32, original_height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
        if original_width == 0 || original_height == 0 {
            return (max_width, max_height);
        }

        let width_ratio = max_width as f64 / original_width as f64;
        let height_ratio = max_height as f64 / original_height as f64;
        let scale_factor = width_ratio.min(height_ratio);

        let new_width = (original_width as f64 * scale_factor).round() as u32;
        let new_height = (original_height as f64 * scale_factor).round() as u64;

        // Ensure dimensions are at least 1 pixel
        (new_width.max(1), new_height.max(1) as u32)
    }

    /// Save thumbnail to file
    fn save_thumbnail(&self, img: &DynamicImage, path: &Path, config: &ThumbnailConfig) -> Result<()> {
        match config.format {
            ThumbnailFormat::JPEG => {
                img.save_with_quality(path, image::JPEG(config.quality))
                    .map_err(|e| anyhow!("Failed to save JPEG thumbnail: {}", e))?;
            }
            ThumbnailFormat::PNG => {
                img.save_with_format(path, ImageFormat::Png)
                    .map_err(|e| anyhow!("Failed to save PNG thumbnail: {}", e))?;
            }
            ThumbnailFormat::WEBP => {
                img.save_with_format(path, ImageFormat::WebP)
                    .map_err(|e| anyhow!("Failed to save WebP thumbnail: {}", e))?;
            }
        }

        Ok(())
    }

    /// Generate thumbnail filename
    fn generate_thumbnail_filename(&self, file_path: &Path) -> String {
        Self::generate_thumbnail_filename_for_path(file_path, &self.config.format)
    }

    /// Generate thumbnail filename for a given path and format
    fn generate_thumbnail_filename_for_path(file_path: &Path, format: &ThumbnailFormat) -> String {
        if let Some(stem) = file_path.file_stem() {
            format!("{}_thumb.{}", stem.to_string_lossy(), format.extension())
        } else {
            // Fallback to UUID if no filename
            format!("{}.{}", uuid::Uuid::new_v4(), format.extension())
        }
    }

    /// Generate multiple thumbnails at different sizes
    pub async fn generate_multiple_thumbnails(
        &mut self,
        video_path: &Path,
        output_dir: &Path,
        sizes: &[(u32, u32)],
    ) -> Result<Vec<ThumbnailResult>> {
        let mut results = Vec::new();

        for &(width, height) in sizes {
            let mut config = self.config.clone();
            config.width = width;
            config.height = height;

            let mut generator = ThumbnailGenerator::new(config);

            let result = generator.generate_video_thumbnail(video_path, output_dir).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Generate thumbnail for audio file (album art or waveform)
    pub async fn generate_audio_thumbnail(&self, audio_path: &Path, output_dir: &Path) -> Result<ThumbnailResult> {
        info!("Generating audio thumbnail for: {}", audio_path.display());

        // For now, generate a simple waveform visualization
        // In a full implementation, you would extract album art or generate waveforms
        self.generate_waveform_thumbnail(audio_path, output_dir).await
    }

    /// Generate waveform thumbnail for audio
    async fn generate_waveform_thumbnail(&self, audio_path: &Path, output_dir: &Path) -> Result<ThumbnailResult> {
        // Create a simple placeholder waveform image
        let width = self.config.width;
        let height = self.config.height;

        // Generate a simple waveform pattern
        let img = tokio::task::spawn_blocking(move || {
            let mut img = RgbImage::new(width, height);

            // Generate simple waveform pattern
            for x in 0..width {
                let amplitude = ((x as f32 / width as f32) * std::f32::consts::PI * 2.0).sin().abs();
                let bar_height = (amplitude * height as f32 * 0.8) as u32;
                let start_y = (height - bar_height) / 2;

                for y in start_y..(start_y + bar_height) {
                    img.put_pixel(x, y, Rgb([100, 150, 200])); // Blue color
                }
            }

            DynamicImage::ImageRgb8(img)
        }).await
            .map_err(|e| anyhow!("Failed to generate waveform: {}", e))?;

        // Save thumbnail
        let filename = self.generate_thumbnail_filename(audio_path);
        let thumbnail_path = output_dir.join(filename);

        tokio::task::spawn_blocking(move || {
            self.save_thumbnail(&img, &thumbnail_path, &self.config)
        }).await
            .map_err(|e| anyhow!("Failed to save audio thumbnail: {}", e))??;

        // Get thumbnail file size
        let file_size = tokio::fs::metadata(&thumbnail_path).await
            .map_err(|e| anyhow!("Failed to get thumbnail metadata: {}", e))?
            .len();

        Ok(ThumbnailResult {
            path: thumbnail_path,
            width: img.width(),
            height: img.height(),
            file_size,
            format: self.config.format.extension().to_string(),
        })
    }

    /// Batch generate thumbnails for multiple files
    pub async fn batch_generate_thumbnails(
        &mut self,
        file_paths: &[PathBuf],
        output_dir: &Path,
    ) -> Result<Vec<Result<ThumbnailResult>>> {
        let mut results = Vec::new();

        for file_path in file_paths {
            let result = match self.detect_media_type(file_path) {
                MediaType::Video => self.generate_video_thumbnail(file_path, output_dir).await,
                MediaType::Image => self.generate_image_thumbnail(file_path, output_dir).await,
                MediaType::Audio => self.generate_audio_thumbnail(file_path, output_dir).await,
                MediaType::Unknown => Err(anyhow!("Unknown media type: {}", file_path.display())),
            };

            results.push(result);
        }

        Ok(results)
    }

    /// Detect media type from file extension
    fn detect_media_type(&self, file_path: &Path) -> MediaType {
        if let Some(ext) = file_path.extension() {
            match ext.to_str().unwrap_or("").to_lowercase().as_str() {
                "mp4" | "avi" | "mov" | "mkv" | "webm" | "wmv" | "flv" | "m4v" => MediaType::Video,
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" => MediaType::Audio,
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" => MediaType::Image,
                _ => MediaType::Unknown,
            }
        } else {
            MediaType::Unknown
        }
    }

    /// Check if thumbnail exists for file
    pub fn thumbnail_exists(&self, file_path: &Path, output_dir: &Path) -> bool {
        let thumbnail_path = output_dir.join(self.generate_thumbnail_filename(file_path));
        thumbnail_path.exists()
    }

    /// Get thumbnail path for file (without generating)
    pub fn get_thumbnail_path(&self, file_path: &Path, output_dir: &Path) -> PathBuf {
        output_dir.join(self.generate_thumbnail_filename(file_path))
    }

    /// Clean up old thumbnails
    pub async fn cleanup_old_thumbnails(&self, output_dir: &Path, max_age_days: u64) -> Result<usize> {
        let mut removed_count = 0;

        if let Ok(mut entries) = tokio::fs::read_dir(output_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                // Check if it's a thumbnail file
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains("_thumb.") {
                        // Check file age
                        if let Ok(metadata) = tokio::fs::metadata(&path).await {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(elapsed) = modified.elapsed() {
                                    if elapsed.as_secs() > max_age_days * 24 * 60 * 60 {
                                        if let Err(e) = tokio::fs::remove_file(&path).await {
                                            warn!("Failed to remove old thumbnail {}: {}", path.display(), e);
                                        } else {
                                            debug!("Removed old thumbnail: {}", path.display());
                                            removed_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(removed_count)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum MediaType {
    Video,
    Audio,
    Image,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_thumbnail_config_default() {
        let config = ThumbnailConfig::default();
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 240);
        assert_eq!(config.quality, 85);
        assert_eq!(config.format.extension(), "jpg");
        assert!(config.maintain_aspect_ratio);
    }

    #[test]
    fn test_calculate_dimensions() {
        // Test maintaining aspect ratio
        let (width, height) = ThumbnailGenerator::calculate_dimensions(1920, 1080, 320, 240);
        assert_eq!(width, 320);
        assert_eq!(height, 180); // 16:9 ratio

        // Test portrait image
        let (width, height) = ThumbnailGenerator::calculate_dimensions(1080, 1920, 320, 240);
        assert_eq!(width, 135);
        assert_eq!(height, 240); // Portrait ratio

        // Test small image
        let (width, height) = ThumbnailGenerator::calculate_dimensions(100, 100, 320, 240);
        assert_eq!(width, 100);
        assert_eq!(height, 100); // Don't upscale
    }

    #[test]
    fn test_thumbnail_filename_generation() {
        let config = ThumbnailConfig::default();
        let generator = ThumbnailGenerator::new(config);

        let path = Path::new("test_video.mp4");
        let filename = generator.generate_thumbnail_filename(path);
        assert_eq!(filename, "test_video_thumb.jpg");

        let path = Path::new("another.movie.mkv");
        let filename = generator.generate_thumbnail_filename(path);
        assert_eq!(filename, "another.movie_thumb.jpg");
    }

    #[test]
    fn test_media_type_detection() {
        let generator = ThumbnailGenerator::new_default();

        assert_eq!(generator.detect_media_type(Path::new("test.mp4")), MediaType::Video);
        assert_eq!(generator.detect_media_type(Path::new("test.avi")), MediaType::Video);
        assert_eq!(generator.detect_media_type(Path::new("test.mp3")), MediaType::Audio);
        assert_eq!(generator.detect_media_type(Path::new("test.wav")), MediaType::Audio);
        assert_eq!(generator.detect_media_type(Path::new("test.jpg")), MediaType::Image);
        assert_eq!(generator.detect_media_type(Path::new("test.png")), MediaType::Image);
        assert_eq!(generator.detect_media_type(Path::new("test.xyz")), MediaType::Unknown);
    }
}