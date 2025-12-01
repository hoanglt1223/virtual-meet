//! Media Library Scanner
//!
//! Background scanning service for discovering and processing media files.
//! Handles recursive directory scanning, metadata extraction, and thumbnail generation.

use anyhow::{Result, anyhow};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::{info, error, debug, warn};
use walkdir::WalkDir;
use uuid::Uuid;

use crate::commands_media::{MediaFileInfo, MediaType, MediaLibraryScanRequest};
use crate::media_library::{MediaLibraryDatabase, ScanHistory};
use crate::metadata_extractor::{MetadataExtractor, MediaAnalysis};
use crate::thumbnail_generator::{ThumbnailGenerator, ThumbnailConfig};

/// Scanner configuration
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub max_concurrent_files: usize,
    pub thumbnail_config: ThumbnailConfig,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_file_size: u64,
    pub enable_thumbnails: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_files: 4,
            thumbnail_config: ThumbnailConfig::default(),
            include_patterns: vec![
                "*.mp4".to_string(),
                "*.avi".to_string(),
                "*.mov".to_string(),
                "*.mkv".to_string(),
                "*.webm".to_string(),
                "*.mp3".to_string(),
                "*.wav".to_string(),
                "*.flac".to_string(),
                "*.jpg".to_string(),
                "*.jpeg".to_string(),
                "*.png".to_string(),
            ],
            exclude_patterns: vec![
                ".*".to_string(), // Hidden files
                "Thumbs.db".to_string(),
                "desktop.ini".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024 * 1024, // 10GB
            enable_thumbnails: true,
        }
    }
}

/// Scan progress update
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub scanned_paths: Vec<String>,
    pub files_found: usize,
    pub files_processed: usize,
    pub current_file: Option<String>,
    pub errors: Vec<String>,
    pub total_size_bytes: u64,
    pub elapsed_ms: u64,
}

/// Scan result
#[derive(Debug)]
pub struct ScanResult {
    pub files_found: Vec<MediaFileInfo>,
    pub errors: Vec<String>,
    pub scanned_paths: Vec<String>,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub scan_duration_ms: u64,
}

/// Media scanner service
pub struct MediaScanner {
    database: Arc<MediaLibraryDatabase>,
    config: ScannerConfig,
    metadata_extractor: Arc<RwLock<MetadataExtractor>>,
    is_scanning: Arc<RwLock<bool>>,
}

impl MediaScanner {
    /// Create new media scanner
    pub fn new(database: Arc<MediaLibraryDatabase>, config: ScannerConfig) -> Self {
        Self {
            database,
            config,
            metadata_extractor: Arc::new(RwLock::new(MetadataExtractor::new())),
            is_scanning: Arc::new(RwLock::new(false)),
        }
    }

    /// Start scanning media library
    pub async fn scan_library(&self, request: MediaLibraryScanRequest) -> Result<ScanResult> {
        info!("Starting media library scan for {:?} paths", request.paths.len());

        // Check if already scanning
        {
            let is_scanning = self.is_scanning.read().await;
            if *is_scanning {
                return Err(anyhow!("Scan already in progress"));
            }
        }

        // Set scanning flag
        {
            let mut is_scanning = self.is_scanning.write().await;
            *is_scanning = true;
        }

        let start_time = Instant::now();
        let mut result = ScanResult {
            files_found: Vec::new(),
            errors: Vec::new(),
            scanned_paths: Vec::new(),
            total_files: 0,
            total_size_bytes: 0,
            scan_duration_ms: 0,
        };

        // Reset scanning flag when done
        let _guard = scopeguard::guard(self.is_scanning.clone(), |is_scanning| {
            tokio::spawn(async move {
                let mut is_scanning = is_scanning.write().await;
                *is_scanning = false;
            });
        });

        let scan_results = self.perform_scan(request).await;

        match scan_results {
            Ok(mut scan_result) => {
                scan_result.scan_duration_ms = start_time.elapsed().as_millis() as u64;
                result = scan_result;

                // Record scan history
                let history = ScanHistory {
                    id: None,
                    scan_path: result.scanned_paths.join("; "),
                    scan_time: Utc::now(),
                    files_found: result.total_files as i32,
                    total_size: result.total_size_bytes as i64,
                    errors: if result.errors.is_empty() {
                        None
                    } else {
                        Some(result.errors.join("; "))
                    },
                    scan_duration_ms: result.scan_duration_ms as i64,
                };

                if let Err(e) = self.database.record_scan_history(&history).await {
                    error!("Failed to record scan history: {}", e);
                }

                info!("Scan completed successfully: {} files found, {} errors",
                      result.total_files, result.errors.len());
            }
            Err(e) => {
                result.errors.push(format!("Scan failed: {}", e));
                result.scan_duration_ms = start_time.elapsed().as_millis() as u64;
                error!("Scan failed: {}", e);
            }
        }

        Ok(result)
    }

    /// Perform the actual scanning
    async fn perform_scan(&self, request: MediaLibraryScanRequest) -> Result<ScanResult> {
        let mut all_files = Vec::new();
        let mut all_errors = Vec::new();
        let mut all_paths = Vec::new();

        for scan_path in &request.paths {
            debug!("Scanning path: {}", scan_path);

            match self.scan_single_path(scan_path, &request).await {
                Ok((mut files, mut errors, paths)) => {
                    all_files.append(&mut files);
                    all_errors.append(&mut errors);
                    all_paths.extend(paths);
                }
                Err(e) => {
                    all_errors.push(format!("Failed to scan {}: {}", scan_path, e));
                }
            }
        }

        // Process files in parallel
        let processed_files = self.process_files_parallel(&all_files).await?;

        let total_size: u64 = processed_files.iter().map(|f| f.size).sum();
        let total_files = processed_files.len();

        Ok(ScanResult {
            files_found: processed_files,
            errors: all_errors,
            scanned_paths: all_paths,
            total_files,
            total_size_bytes: total_size,
            scan_duration_ms: 0, // Will be set by caller
        })
    }

    /// Scan a single path
    async fn scan_single_path(
        &self,
        scan_path: &str,
        request: &MediaLibraryScanRequest,
    ) -> Result<(Vec<PathBuf>, Vec<String>, Vec<String>)> {
        let path = Path::new(scan_path);
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", scan_path));
        }

        let mut files = Vec::new();
        let mut errors = Vec::new();
        let mut scanned_paths = Vec::new();

        info!("Walking directory tree: {}", scan_path);

        let walker = WalkDir::new(path)
            .follow_links(false)
            .max_depth(if request.recursive { usize::MAX } else { 1 })
            .into_iter();

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let entry_path = entry.path();

                    if entry_path.is_dir() {
                        scanned_paths.push(entry_path.to_string_lossy().to_string());
                        continue;
                    }

                    if self.should_process_file(entry_path) {
                        files.push(entry_path.to_path_buf());
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to read directory entry: {}", e));
                    warn!("Directory walk error: {}", e);
                }
            }
        }

        Ok((files, errors, scanned_paths))
    }

    /// Check if file should be processed
    fn should_process_file(&self, file_path: &Path) -> bool {
        // Check file size
        if let Ok(metadata) = std::fs::metadata(file_path) {
            if metadata.len() > self.config.max_file_size {
                debug!("Skipping large file: {} ({} bytes)", file_path.display(), metadata.len());
                return false;
            }
        } else {
            debug!("Cannot read metadata for: {}", file_path.display());
            return false;
        }

        // Check include patterns
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            let included = self.config.include_patterns.iter().any(|pattern| {
                self.matches_pattern(file_name, pattern)
            });

            if !included {
                return false;
            }

            // Check exclude patterns
            let excluded = self.config.exclude_patterns.iter().any(|pattern| {
                self.matches_pattern(file_name, pattern)
            });

            if excluded {
                debug!("Excluding file: {}", file_path.display());
                return false;
            }
        }

        true
    }

    /// Simple pattern matching (supports wildcards)
    fn matches_pattern(&self, filename: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Convert wildcard pattern to regex
            let regex_pattern = pattern
                .replace('.', r"\.")
                .replace('*', ".*")
                .replace('?', ".");

            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                regex.is_match(filename)
            } else {
                false
            }
        } else {
            filename.to_lowercase().contains(&pattern.to_lowercase())
        }
    }

    /// Process files in parallel
    async fn process_files_parallel(&self, file_paths: &[PathBuf]) -> Result<Vec<MediaFileInfo>> {
        info!("Processing {} files with {} workers",
              file_paths.len(), self.config.max_concurrent_files);

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_files));
        let mut tasks = Vec::new();

        for file_path in file_paths {
            let file_path = file_path.clone();
            let semaphore = semaphore.clone();
            let database = self.database.clone();
            let metadata_extractor = self.metadata_extractor.clone();
            let thumbnail_config = self.config.thumbnail_config.clone();
            let enable_thumbnails = self.config.enable_thumbnails;

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                self.process_single_file(
                    &file_path,
                    &database,
                    &metadata_extractor,
                    &thumbnail_config,
                    enable_thumbnails,
                ).await
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(Some(media_info))) => results.push(media_info),
                Ok(Ok(None)) => {}, // File was skipped
                Ok(Err(e)) => warn!("Failed to process file: {}", e),
                Err(e) => error!("Task failed: {}", e),
            }
        }

        info!("Successfully processed {} files", results.len());
        Ok(results)
    }

    /// Process a single file
    async fn process_single_file(
        &self,
        file_path: &Path,
        database: &Arc<MediaLibraryDatabase>,
        metadata_extractor: &Arc<RwLock<MetadataExtractor>>,
        thumbnail_config: &ThumbnailConfig,
        enable_thumbnails: bool,
    ) -> Result<Option<MediaFileInfo>> {
        debug!("Processing file: {}", file_path.display());

        // Check if file already exists in database
        if let Ok(Some(db_file)) = database.get_media_file_by_path(&file_path.to_string_lossy()).await {
            // Check if file was modified since last scan
            if let Ok(metadata) = tokio::fs::metadata(file_path).await {
                if let Ok(modified) = metadata.modified() {
                    if let Some(db_modified) = db_file.modified_at.duration_since(std::time::UNIX_EPOCH).ok() {
                        let modified_time = chrono::DateTime::from_timestamp(modified.as_secs() as i64, modified.subsec_nanos());
                        if let Some(modified_time) = modified_time {
                            if modified_time <= db_file.modified_at {
                                debug!("File unchanged, skipping: {}", file_path.display());
                                return Ok(None); // Skip unchanged file
                            }
                        }
                    }
                }
            }
        }

        // Extract metadata
        let media_analysis = {
            let mut extractor = metadata_extractor.write().await;
            extractor.analyze_media_file(file_path).await
        };

        let media_analysis = match media_analysis {
            Ok(analysis) => analysis,
            Err(e) => {
                warn!("Failed to extract metadata from {}: {}", file_path.display(), e);
                return Ok(None);
            }
        };

        // Determine media type
        let media_type = if !media_analysis.video_streams.is_empty() {
            MediaType::Video
        } else if !media_analysis.audio_streams.is_empty() {
            MediaType::Audio
        } else {
            MediaType::Unknown
        };

        if media_type == MediaType::Unknown {
            debug!("Skipping unknown media type: {}", file_path.display());
            return Ok(None);
        }

        // Generate thumbnail if enabled and applicable
        let thumbnail_path = if enable_thumbnails && (media_type == MediaType::Video || media_type == MediaType::Audio) {
            self.generate_thumbnail_for_file(file_path, &media_type, thumbnail_config, database).await?
        } else {
            None
        };

        // Create media file info
        let file_metadata = tokio::fs::metadata(file_path).await?;
        let file_name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let media_info = MediaFileInfo {
            path: file_path.to_string_lossy().to_string(),
            name: file_name,
            file_type: media_type,
            size: file_metadata.len(),
            duration: Some(media_analysis.duration),
            metadata: metadata_extractor.read().await.to_media_metadata(&media_analysis),
            thumbnail_path,
            last_modified: file_metadata.modified()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .map(|d| format!("{} seconds ago", d.as_secs()))
                .unwrap_or_else(|| "Unknown".to_string()),
            created: file_metadata.created()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .map(|d| format!("{} seconds ago", d.as_secs()))
                .unwrap_or_else(|| "Unknown".to_string()),
        };

        // Store in database
        if let Err(e) = database.upsert_media_file(&media_info).await {
            error!("Failed to store media file in database: {}", e);
        }

        debug!("Successfully processed: {}", file_path.display());
        Ok(Some(media_info))
    }

    /// Generate thumbnail for file
    async fn generate_thumbnail_for_file(
        &self,
        file_path: &Path,
        media_type: &MediaType,
        thumbnail_config: &ThumbnailConfig,
        database: &Arc<MediaLibraryDatabase>,
    ) -> Result<Option<String>> {
        let thumbnail_dir = database.get_thumbnail_path();

        // Check if thumbnail already exists
        let mut generator = ThumbnailGenerator::new(thumbnail_config.clone());
        if generator.thumbnail_exists(file_path, thumbnail_dir) {
            let thumbnail_path = generator.get_thumbnail_path(file_path, thumbnail_dir);
            return Ok(Some(thumbnail_path.to_string_lossy().to_string()));
        }

        // Generate thumbnail
        let thumbnail_result = match media_type {
            MediaType::Video => {
                generator.generate_video_thumbnail(file_path, thumbnail_dir).await
            }
            MediaType::Audio => {
                generator.generate_audio_thumbnail(file_path, thumbnail_dir).await
            }
            _ => return Ok(None),
        };

        match thumbnail_result {
            Ok(result) => {
                debug!("Generated thumbnail: {}", result.path.display());
                Ok(Some(result.path.to_string_lossy().to_string()))
            }
            Err(e) => {
                warn!("Failed to generate thumbnail for {}: {}", file_path.display(), e);
                Ok(None)
            }
        }
    }

    /// Check if currently scanning
    pub async fn is_scanning(&self) -> bool {
        *self.is_scanning.read().await
    }

    /// Get scan progress (placeholder for future implementation)
    pub async fn get_scan_progress(&self) -> ScanProgress {
        ScanProgress {
            scanned_paths: Vec::new(),
            files_found: 0,
            files_processed: 0,
            current_file: None,
            errors: Vec::new(),
            total_size_bytes: 0,
            elapsed_ms: 0,
        }
    }

    /// Cancel ongoing scan
    pub async fn cancel_scan(&self) -> Result<()> {
        // TODO: Implement scan cancellation
        info!("Scan cancellation requested");
        Ok(())
    }

    /// Clean up orphaned files in database
    pub async fn cleanup_orphaned_files(&self) -> Result<usize> {
        info!("Cleaning up orphaned files in database");

        // This would find files in the database that no longer exist on disk
        // and remove them. For now, just clean up thumbnails.
        let removed_thumbnails = self.database.cleanup_orphaned_thumbnails().await?;

        info!("Cleaned up {} orphaned thumbnails", removed_thumbnails);
        Ok(removed_thumbnails)
    }
}

// Add regex dependency for pattern matching
use regex;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_pattern_matching() {
        let config = ScannerConfig::default();
        let scanner = MediaScanner::new(
            Arc::new(MediaLibraryDatabase::new(
                &TempDir::new().unwrap().path().join("test.db"),
                &TempDir::new().unwrap().path(),
            ).await.unwrap()),
            config,
        );

        assert!(scanner.matches_pattern("test.mp4", "*.mp4"));
        assert!(scanner.matches_pattern("test.MP4", "*.mp4")); // Case insensitive
        assert!(scanner.matches_pattern("video.mp4", "video.*"));
        assert!(!scanner.matches_pattern("test.avi", "*.mp4"));
    }

    #[test]
    fn test_should_process_file() {
        let config = ScannerConfig::default();
        let scanner = MediaScanner::new(
            Arc::new(MediaLibraryDatabase::new(
                &TempDir::new().unwrap().path().join("test.db"),
                &TempDir::new().unwrap().path(),
            ).await.unwrap()),
            config,
        );

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.mp4");
        fs::write(&test_file, "dummy content").unwrap();

        assert!(scanner.should_process_file(&test_file));

        // Test exclusion
        let hidden_file = temp_dir.path().join(".hidden.mp4");
        fs::write(&hidden_file, "dummy content").unwrap();
        assert!(!scanner.should_process_file(&hidden_file));
    }

    #[test]
    fn test_scanner_config_default() {
        let config = ScannerConfig::default();
        assert_eq!(config.max_concurrent_files, 4);
        assert!(config.enable_thumbnails);
        assert!(!config.include_patterns.is_empty());
        assert!(!config.exclude_patterns.is_empty());
        assert_eq!(config.max_file_size, 10 * 1024 * 1024 * 1024); // 10GB
    }
}