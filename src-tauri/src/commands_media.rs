//! Media Control Commands
//!
//! Tauri commands for media library management and file operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{command, State};
use tracing::{info, error, warn, debug};

use crate::AppState;
use crate::audio::AudioMetadata;
use crate::virtual::VideoInfo;

/// Media file information
#[derive(Debug, Serialize, Clone)]
pub struct MediaFileInfo {
    pub path: String,
    pub name: String,
    pub file_type: MediaType,
    pub size: u64,
    pub duration: Option<f64>,
    pub metadata: MediaMetadata,
    pub thumbnail_path: Option<String>,
    pub last_modified: String,
    pub created: String,
}

/// Media type enumeration
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum MediaType {
    Video,
    Audio,
    Image,
    Unknown,
}

/// Media metadata container
#[derive(Debug, Serialize, Clone)]
pub struct MediaMetadata {
    pub video_info: Option<VideoInfo>,
    pub audio_info: Option<AudioMetadata>,
    pub format_info: FormatInfo,
}

/// Format information
#[derive(Debug, Serialize, Clone)]
pub struct FormatInfo {
    pub format_name: String,
    pub duration: Option<f64>,
    pub bit_rate: Option<u64>,
    pub file_size: u64,
}

/// Media library scan request
#[derive(Debug, Deserialize)]
pub struct MediaLibraryScanRequest {
    pub paths: Vec<String>,
    pub recursive: bool,
    pub include_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub generate_thumbnails: bool,
}

/// Media library scan response
#[derive(Debug, Serialize)]
pub struct MediaLibraryScanResponse {
    pub success: bool,
    pub message: String,
    pub scanned_paths: Vec<String>,
    pub found_files: Vec<MediaFileInfo>,
    pub errors: Vec<String>,
    pub scan_duration_ms: u64,
    pub total_files_found: usize,
    pub total_size_bytes: u64,
}

/// Media library status response
#[derive(Debug, Serialize)]
pub struct MediaLibraryStatusResponse {
    pub success: bool,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub media_counts: MediaCounts,
    pub last_scan_time: Option<String>,
    pub library_paths: Vec<String>,
}

/// Media type counts
#[derive(Debug, Serialize)]
pub struct MediaCounts {
    pub video_files: usize,
    pub audio_files: usize,
    pub image_files: usize,
    pub other_files: usize,
}

/// Load media library request
#[derive(Debug, Deserialize)]
pub struct LoadMediaLibraryRequest {
    pub library_path: String,
    pub force_refresh: bool,
    pub include_subdirectories: bool,
}

/// Set current media request
#[derive(Debug, Deserialize)]
pub struct SetCurrentMediaRequest {
    pub video_path: Option<String>,
    pub audio_path: Option<String>,
    pub auto_play: bool,
}

/// Media validation result
#[derive(Debug, Serialize)]
pub struct MediaValidationResult {
    pub is_valid: bool,
    pub file_type: MediaType,
    pub can_decode: bool,
    pub duration: Option<f64>,
    pub metadata: Option<MediaMetadata>,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
}

/// Get supported media formats response
#[derive(Debug, Serialize)]
pub struct SupportedFormatsResponse {
    pub success: bool,
    pub video_formats: Vec<String>,
    pub audio_formats: Vec<String>,
    pub image_formats: Vec<String>,
    pub container_formats: Vec<String>,
}

/// Media library search request
#[derive(Debug, Deserialize)]
pub struct MediaSearchRequest {
    pub query: String,
    pub search_type: SearchType,
    pub file_types: Option<Vec<MediaType>>,
    pub max_results: Option<usize>,
}

/// Search type enumeration
#[derive(Debug, Deserialize)]
pub enum SearchType {
    Filename,
    FullPath,
    Metadata,
    All,
}

/// Media library search response
#[derive(Debug, Serialize)]
pub struct MediaSearchResponse {
    pub success: bool,
    pub results: Vec<MediaFileInfo>,
    pub total_matches: usize,
    pub search_time_ms: u64,
}

/// Load media library
#[command]
pub async fn load_media_library(
    request: LoadMediaLibraryRequest,
    state: State<'_, AppState>,
) -> Result<MediaLibraryScanResponse, String> {
    info!("Loading media library from: {}", request.library_path);

    let start_time = std::time::Instant::now();
    let library_path = Path::new(&request.library_path);

    if !library_path.exists() {
        return Ok(MediaLibraryScanResponse {
            success: false,
            message: format!("Library path does not exist: {}", request.library_path),
            scanned_paths: vec![],
            found_files: vec![],
            errors: vec![],
            scan_duration_ms: start_time.elapsed().as_millis() as u64,
            total_files_found: 0,
            total_size_bytes: 0,
        });
    }

    let mut found_files = Vec::new();
    let mut errors = Vec::new();
    let mut scanned_paths = Vec::new();

    // Scan for media files
    if let Err(e) = scan_directory_for_media(
        library_path,
        request.include_subdirectories,
        &mut found_files,
        &mut errors,
        &mut scanned_paths,
    ) {
        error!("Failed to scan media library: {}", e);
        return Ok(MediaLibraryScanResponse {
            success: false,
            message: format!("Failed to scan library: {}", e),
            scanned_paths,
            found_files,
            errors,
            scan_duration_ms: start_time.elapsed().as_millis() as u64,
            total_files_found: 0,
            total_size_bytes: 0,
        });
    }

    let total_size_bytes: u64 = found_files.iter().map(|f| f.size).sum();
    let total_files_found = found_files.len();

    info!(
        "Media library scan completed: {} files found, {} total size",
        total_files_found,
        format_bytes(total_size_bytes)
    );

    Ok(MediaLibraryScanResponse {
        success: true,
        message: format!("Successfully loaded {} media files", total_files_found),
        scanned_paths,
        found_files,
        errors,
        scan_duration_ms: start_time.elapsed().as_millis() as u64,
        total_files_found,
        total_size_bytes,
    })
}

/// Set current media
#[command]
pub async fn set_current_video(
    video_path: String,
    state: State<'_, AppState>,
) -> Result<MediaValidationResult, String> {
    info!("Setting current video: {}", video_path);

    let path = Path::new(&video_path);
    if !path.exists() {
        return Ok(MediaValidationResult {
            is_valid: false,
            file_type: MediaType::Unknown,
            can_decode: false,
            duration: None,
            metadata: None,
            error_message: Some(format!("Video file does not exist: {}", video_path)),
            warnings: vec![],
        });
    }

    // Validate video file
    match validate_media_file(&video_path, MediaType::Video).await {
        Ok(result) => {
            if result.is_valid && result.can_decode {
                info!("Video file validated successfully: {}", video_path);
                // In a real implementation, you would load this into the pipeline
            }
            Ok(result)
        },
        Err(e) => Ok(MediaValidationResult {
            is_valid: false,
            file_type: MediaType::Video,
            can_decode: false,
            duration: None,
            metadata: None,
            error_message: Some(format!("Failed to validate video: {}", e)),
            warnings: vec![],
        })
    }
}

/// Set current audio
#[command]
pub async fn set_current_audio(
    audio_path: String,
    state: State<'_, AppState>,
) -> Result<MediaValidationResult, String> {
    info!("Setting current audio: {}", audio_path);

    let path = Path::new(&audio_path);
    if !path.exists() {
        return Ok(MediaValidationResult {
            is_valid: false,
            file_type: MediaType::Unknown,
            can_decode: false,
            duration: None,
            metadata: None,
            error_message: Some(format!("Audio file does not exist: {}", audio_path)),
            warnings: vec![],
        });
    }

    // Validate audio file
    match validate_media_file(&audio_path, MediaType::Audio).await {
        Ok(result) => {
            if result.is_valid && result.can_decode {
                info!("Audio file validated successfully: {}", audio_path);
                // In a real implementation, you would load this into the pipeline
            }
            Ok(result)
        },
        Err(e) => Ok(MediaValidationResult {
            is_valid: false,
            file_type: MediaType::Audio,
            can_decode: false,
            duration: None,
            metadata: None,
            error_message: Some(format!("Failed to validate audio: {}", e)),
            warnings: vec![],
        })
    }
}

/// Get supported media formats
#[command]
pub async fn get_supported_media_formats() -> Result<SupportedFormatsResponse, String> {
    info!("Getting supported media formats");

    let video_formats = vec![
        "mp4".to_string(),
        "avi".to_string(),
        "mov".to_string(),
        "mkv".to_string(),
        "webm".to_string(),
        "wmv".to_string(),
        "flv".to_string(),
        "m4v".to_string(),
    ];

    let audio_formats = vec![
        "mp3".to_string(),
        "wav".to_string(),
        "flac".to_string(),
        "aac".to_string(),
        "ogg".to_string(),
        "m4a".to_string(),
        "wma".to_string(),
        "opus".to_string(),
    ];

    let image_formats = vec![
        "jpg".to_string(),
        "jpeg".to_string(),
        "png".to_string(),
        "gif".to_string(),
        "bmp".to_string(),
        "tiff".to_string(),
        "webp".to_string(),
    ];

    let container_formats = vec![
        "mp4".to_string(),
        "avi".to_string(),
        "mov".to_string(),
        "mkv".to_string(),
        "webm".to_string(),
    ];

    Ok(SupportedFormatsResponse {
        success: true,
        video_formats,
        audio_formats,
        image_formats,
        container_formats,
    })
}

/// Search media library
#[command]
pub async fn search_media_library(
    request: MediaSearchRequest,
) -> Result<MediaSearchResponse, String> {
    info!("Searching media library for: {}", request.query);

    let start_time = std::time::Instant::now();

    // This is a placeholder implementation
    // In a real implementation, you would search through the loaded media library
    let results = Vec::new(); // Placeholder

    Ok(MediaSearchResponse {
        success: true,
        results,
        total_matches: 0,
        search_time_ms: start_time.elapsed().as_millis() as u64,
    })
}

/// Get media library status
#[command]
pub async fn get_media_library_status() -> Result<MediaLibraryStatusResponse, String> {
    info!("Getting media library status");

    // This is a placeholder implementation
    // In a real implementation, you would return the current library status
    Ok(MediaLibraryStatusResponse {
        success: true,
        total_files: 0,
        total_size_bytes: 0,
        media_counts: MediaCounts {
            video_files: 0,
            audio_files: 0,
            image_files: 0,
            other_files: 0,
        },
        last_scan_time: None,
        library_paths: vec![],
    })
}

/// Validate media file
async fn validate_media_file(
    file_path: &str,
    expected_type: MediaType,
) -> Result<MediaValidationResult> {
    // This is a placeholder implementation
    // In a real implementation, you would use FFmpeg or other libraries to validate the file

    Ok(MediaValidationResult {
        is_valid: true,
        file_type: expected_type,
        can_decode: true,
        duration: Some(0.0),
        metadata: None,
        error_message: None,
        warnings: vec![],
    })
}

/// Scan directory for media files
fn scan_directory_for_media(
    dir_path: &Path,
    recursive: bool,
    found_files: &mut Vec<MediaFileInfo>,
    errors: &mut Vec<String>,
    scanned_paths: &mut Vec<String>,
) -> Result<()> {
    scanned_paths.push(dir_path.to_string_lossy().to_string());

    let entries = std::fs::read_dir(dir_path)
        .map_err(|e| anyhow::anyhow!("Failed to read directory {}: {}", dir_path.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            anyhow::anyhow!("Failed to read directory entry in {}: {}", dir_path.display(), e)
        })?;

        let path = entry.path();

        if path.is_dir() && recursive {
            if let Err(e) = scan_directory_for_media(
                &path,
                recursive,
                found_files,
                errors,
                scanned_paths,
            ) {
                errors.push(format!("Failed to scan directory {}: {}", path.display(), e));
            }
        } else if path.is_file() {
            if let Some(media_info) = create_media_file_info(&path) {
                found_files.push(media_info);
            }
        }
    }

    Ok(())
}

/// Create media file info from path
fn create_media_file_info(path: &Path) -> Option<MediaFileInfo> {
    let metadata = std::fs::metadata(path).ok()?;
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let extension = path.extension()?.to_string_lossy().to_lowercase();

    let file_type = match extension.as_str() {
        "mp4" | "avi" | "mov" | "mkv" | "webm" | "wmv" | "flv" | "m4v" => MediaType::Video,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" => MediaType::Audio,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" => MediaType::Image,
        _ => MediaType::Unknown,
    };

    // Only include known media types
    if file_type == MediaType::Unknown {
        return None;
    }

    let size = metadata.len();
    let last_modified = metadata.modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| format!("{} seconds ago", d.as_secs()))
        .unwrap_or_else(|| "Unknown".to_string());

    let created = metadata.created()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| format!("{} seconds ago", d.as_secs()))
        .unwrap_or_else(|| "Unknown".to_string());

    Some(MediaFileInfo {
        path: path.to_string_lossy().to_string(),
        name: file_name,
        file_type,
        size,
        duration: None,
        metadata: MediaMetadata {
            video_info: None,
            audio_info: None,
            format_info: FormatInfo {
                format_name: extension.clone(),
                duration: None,
                bit_rate: None,
                file_size: size,
            },
        },
        thumbnail_path: None,
        last_modified,
        created,
    })
}

/// Format bytes to human readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_media_type_detection() {
        assert_eq!(
            create_media_file_info(Path::new("test.mp4")).map(|f| f.file_type),
            Some(MediaType::Video)
        );
        assert_eq!(
            create_media_file_info(Path::new("test.mp3")).map(|f| f.file_type),
            Some(MediaType::Audio)
        );
        assert_eq!(
            create_media_file_info(Path::new("test.jpg")).map(|f| f.file_type),
            Some(MediaType::Image)
        );
        assert_eq!(
            create_media_file_info(Path::new("test.xyz")).map(|f| f.file_type),
            None
        );
    }
}