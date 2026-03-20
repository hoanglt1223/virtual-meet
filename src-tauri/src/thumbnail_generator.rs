//! Thumbnail Generator
//!
//! Generates thumbnails for video and image files using ffmpeg CLI and the
//! image crate. No ffmpeg-next bindings required.

use anyhow::{anyhow, Result};
use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::metadata_extractor::MetadataExtractor;

/// Thumbnail configuration
#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    pub width: u32,
    pub height: u32,
    pub quality: u8, // 1-100, for JPEG
    pub format: ThumbnailFormat,
    pub maintain_aspect_ratio: bool,
}

/// Thumbnail output format
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

    pub fn image_format(&self) -> ImageFormat {
        match self {
            ThumbnailFormat::JPEG => ImageFormat::Jpeg,
            ThumbnailFormat::PNG => ImageFormat::Png,
            ThumbnailFormat::WEBP => ImageFormat::WebP,
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

/// Thumbnail generation result
#[derive(Debug)]
pub struct ThumbnailResult {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub format: String,
}

/// Thumbnail generator using ffmpeg CLI
pub struct ThumbnailGenerator {
    metadata_extractor: MetadataExtractor,
    config: ThumbnailConfig,
}

impl ThumbnailGenerator {
    pub fn new(config: ThumbnailConfig) -> Self {
        Self {
            metadata_extractor: MetadataExtractor::new(),
            config,
        }
    }

    pub fn new_default() -> Self {
        Self::new(ThumbnailConfig::default())
    }

    /// Generate thumbnail for a video file using ffmpeg CLI
    pub async fn generate_video_thumbnail(
        &mut self,
        video_path: &Path,
        output_dir: &Path,
    ) -> Result<ThumbnailResult> {
        info!("Generating video thumbnail for: {}", video_path.display());

        if !video_path.exists() {
            return Err(anyhow!("Video file does not exist: {}", video_path.display()));
        }

        let analysis = self
            .metadata_extractor
            .analyze_media_file(video_path)
            .map_err(|e| anyhow!("Failed to analyze video: {}", e))?;

        if analysis.video_streams.is_empty() {
            return Err(anyhow!("No video streams found in file"));
        }

        let video_info = &analysis.video_streams[0];
        let seek_time = self.metadata_extractor.get_thumbnail_position(&analysis);
        let (target_w, target_h) = if self.config.maintain_aspect_ratio {
            Self::calculate_dimensions(
                video_info.width,
                video_info.height,
                self.config.width,
                self.config.height,
            )
        } else {
            (self.config.width, self.config.height)
        };

        debug!("Extracting thumbnail at {:.2}s, {}x{}", seek_time, target_w, target_h);

        let filename = Self::make_thumbnail_filename(video_path, &self.config.format);
        let thumbnail_path = output_dir.join(&filename);

        let seek_str = format!("{:.3}", seek_time);
        let scale_filter = format!("scale={}:{}", target_w, target_h);
        let quality_str = self.config.quality.to_string();
        let thumb_path_str = thumbnail_path.to_string_lossy().to_string();
        let video_path_str = video_path.to_string_lossy().to_string();

        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("ffmpeg")
                .args([
                    "-ss", &seek_str,
                    "-i", &video_path_str,
                    "-vframes", "1",
                    "-vf", &scale_filter,
                    "-q:v", &quality_str,
                    "-y", &thumb_path_str,
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow!("ffmpeg failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("ffmpeg thumbnail failed: {}", stderr));
        }

        let file_size = tokio::fs::metadata(&thumbnail_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(ThumbnailResult {
            path: thumbnail_path,
            width: target_w,
            height: target_h,
            file_size,
            format: self.config.format.extension().to_string(),
        })
    }

    /// Generate thumbnail for an image file
    pub async fn generate_image_thumbnail(
        &self,
        image_path: &Path,
        output_dir: &Path,
    ) -> Result<ThumbnailResult> {
        info!("Generating image thumbnail for: {}", image_path.display());

        if !image_path.exists() {
            return Err(anyhow!("Image file does not exist: {}", image_path.display()));
        }

        let image_path_buf = image_path.to_path_buf();
        let img = tokio::task::spawn_blocking(move || image::open(&image_path_buf))
            .await
            .map_err(|e| anyhow!("Task join error: {}", e))?
            .map_err(|e| anyhow!("Failed to open image: {}", e))?;

        let (orig_w, orig_h) = img.dimensions();
        let (target_w, target_h) = if self.config.maintain_aspect_ratio {
            Self::calculate_dimensions(orig_w, orig_h, self.config.width, self.config.height)
        } else {
            (self.config.width, self.config.height)
        };

        let thumb_img = img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);
        let filename = Self::make_thumbnail_filename(image_path, &self.config.format);
        let thumbnail_path = output_dir.join(&filename);
        let fmt = self.config.format.image_format();
        let thumb_path_clone = thumbnail_path.clone();

        tokio::task::spawn_blocking(move || thumb_img.save_with_format(&thumb_path_clone, fmt))
            .await
            .map_err(|e| anyhow!("Task join error: {}", e))?
            .map_err(|e| anyhow!("Failed to save thumbnail: {}", e))?;

        let file_size = tokio::fs::metadata(&thumbnail_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(ThumbnailResult {
            path: thumbnail_path,
            width: target_w,
            height: target_h,
            file_size,
            format: self.config.format.extension().to_string(),
        })
    }

    /// Generate a simple waveform placeholder thumbnail for audio
    pub async fn generate_audio_thumbnail(
        &self,
        audio_path: &Path,
        output_dir: &Path,
    ) -> Result<ThumbnailResult> {
        info!("Generating audio thumbnail for: {}", audio_path.display());

        let width = self.config.width;
        let height = self.config.height;

        let img = tokio::task::spawn_blocking(move || {
            let mut img = RgbImage::new(width, height);
            for x in 0..width {
                let amplitude = ((x as f32 / width as f32) * std::f32::consts::PI * 2.0)
                    .sin()
                    .abs();
                let bar_height = (amplitude * height as f32 * 0.8) as u32;
                let start_y = (height - bar_height) / 2;
                for y in start_y..(start_y + bar_height) {
                    img.put_pixel(x, y, Rgb([100, 150, 200]));
                }
            }
            DynamicImage::ImageRgb8(img)
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?;

        let filename = Self::make_thumbnail_filename(audio_path, &self.config.format);
        let thumbnail_path = output_dir.join(&filename);
        let fmt = self.config.format.image_format();
        let thumb_path_clone = thumbnail_path.clone();

        tokio::task::spawn_blocking(move || img.save_with_format(&thumb_path_clone, fmt))
            .await
            .map_err(|e| anyhow!("Task join error: {}", e))?
            .map_err(|e| anyhow!("Failed to save audio thumbnail: {}", e))?;

        let file_size = tokio::fs::metadata(&thumbnail_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(ThumbnailResult {
            path: thumbnail_path,
            width,
            height,
            file_size,
            format: self.config.format.extension().to_string(),
        })
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
            results.push(generator.generate_video_thumbnail(video_path, output_dir).await?);
        }
        Ok(results)
    }

    /// Batch generate thumbnails for multiple files
    pub async fn batch_generate_thumbnails(
        &mut self,
        file_paths: &[PathBuf],
        output_dir: &Path,
    ) -> Result<Vec<Result<ThumbnailResult>>> {
        let mut results = Vec::new();
        for file_path in file_paths {
            let result = match Self::detect_media_type(file_path) {
                LocalMediaType::Video => self.generate_video_thumbnail(file_path, output_dir).await,
                LocalMediaType::Image => self.generate_image_thumbnail(file_path, output_dir).await,
                LocalMediaType::Audio => self.generate_audio_thumbnail(file_path, output_dir).await,
                LocalMediaType::Unknown => Err(anyhow!("Unknown media type: {}", file_path.display())),
            };
            results.push(result);
        }
        Ok(results)
    }

    /// Calculate target dimensions maintaining aspect ratio
    pub fn calculate_dimensions(
        original_width: u32,
        original_height: u32,
        max_width: u32,
        max_height: u32,
    ) -> (u32, u32) {
        if original_width == 0 || original_height == 0 {
            return (max_width, max_height);
        }

        // Don't upscale
        if original_width <= max_width && original_height <= max_height {
            return (original_width, original_height);
        }

        let width_ratio = max_width as f64 / original_width as f64;
        let height_ratio = max_height as f64 / original_height as f64;
        let scale = width_ratio.min(height_ratio);

        let new_width = ((original_width as f64 * scale).round() as u32).max(1);
        let new_height = ((original_height as f64 * scale).round() as u32).max(1);
        (new_width, new_height)
    }

    /// Generate thumbnail filename from source path
    pub fn make_thumbnail_filename(file_path: &Path, format: &ThumbnailFormat) -> String {
        if let Some(stem) = file_path.file_stem() {
            format!("{}_thumb.{}", stem.to_string_lossy(), format.extension())
        } else {
            format!("{}.{}", uuid::Uuid::new_v4(), format.extension())
        }
    }

    fn generate_thumbnail_filename(&self, file_path: &Path) -> String {
        Self::make_thumbnail_filename(file_path, &self.config.format)
    }

    pub fn thumbnail_exists(&self, file_path: &Path, output_dir: &Path) -> bool {
        output_dir.join(self.generate_thumbnail_filename(file_path)).exists()
    }

    pub fn get_thumbnail_path(&self, file_path: &Path, output_dir: &Path) -> PathBuf {
        output_dir.join(self.generate_thumbnail_filename(file_path))
    }

    pub async fn cleanup_old_thumbnails(
        &self,
        output_dir: &Path,
        max_age_days: u64,
    ) -> Result<usize> {
        let mut removed = 0;
        if let Ok(mut entries) = tokio::fs::read_dir(output_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.contains("_thumb."))
                    .unwrap_or(false)
                {
                    if let Ok(meta) = tokio::fs::metadata(&path).await {
                        if let Ok(modified) = meta.modified() {
                            if let Ok(elapsed) = modified.elapsed() {
                                if elapsed.as_secs() > max_age_days * 86400 {
                                    if tokio::fs::remove_file(&path).await.is_ok() {
                                        debug!("Removed old thumbnail: {}", path.display());
                                        removed += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(removed)
    }

    fn detect_media_type(file_path: &Path) -> LocalMediaType {
        match file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
            "mp4" | "avi" | "mov" | "mkv" | "webm" | "wmv" | "flv" | "m4v" => LocalMediaType::Video,
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" => LocalMediaType::Audio,
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" => LocalMediaType::Image,
            _ => LocalMediaType::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LocalMediaType {
    Video,
    Audio,
    Image,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (w, h) = ThumbnailGenerator::calculate_dimensions(1920, 1080, 320, 240);
        assert_eq!(w, 320);
        assert_eq!(h, 180);

        let (w, h) = ThumbnailGenerator::calculate_dimensions(1080, 1920, 320, 240);
        assert_eq!(w, 135);
        assert_eq!(h, 240);

        // Small image — don't upscale
        let (w, h) = ThumbnailGenerator::calculate_dimensions(100, 100, 320, 240);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_thumbnail_filename_generation() {
        let filename = ThumbnailGenerator::make_thumbnail_filename(
            Path::new("test_video.mp4"),
            &ThumbnailFormat::JPEG,
        );
        assert_eq!(filename, "test_video_thumb.jpg");
    }

    #[test]
    fn test_media_type_detection() {
        assert_eq!(ThumbnailGenerator::detect_media_type(Path::new("test.mp4")), LocalMediaType::Video);
        assert_eq!(ThumbnailGenerator::detect_media_type(Path::new("test.mp3")), LocalMediaType::Audio);
        assert_eq!(ThumbnailGenerator::detect_media_type(Path::new("test.jpg")), LocalMediaType::Image);
        assert_eq!(ThumbnailGenerator::detect_media_type(Path::new("test.xyz")), LocalMediaType::Unknown);
    }
}
