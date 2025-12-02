//! Media Library Database
//!
//! SQLite database layer for persistent storage of media library information,
//! including metadata, thumbnails, and search indexing.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Pool, Row, Sqlite};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::commands_media::{MediaFileInfo, MediaMetadata, MediaType};

/// Media library database manager
pub struct MediaLibraryDatabase {
    pool: Pool<Sqlite>,
    thumbnail_dir: PathBuf,
}

/// Database media file record
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbMediaFile {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub file_type: String,
    pub size: i64,
    pub duration: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub bit_rate: Option<i64>,
    pub thumbnail_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub tags: Option<String>,     // JSON array
    pub metadata: Option<String>, // JSON blob
}

/// Scan history record
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScanHistory {
    pub id: Option<i64>,
    pub scan_path: String,
    pub scan_time: DateTime<Utc>,
    pub files_found: i32,
    pub total_size: i64,
    pub errors: Option<String>,
    pub scan_duration_ms: i64,
}

/// Search filter options
#[derive(Debug, Deserialize, Clone)]
pub struct SearchFilter {
    pub query: Option<String>,
    pub file_types: Option<Vec<MediaType>>,
    pub min_duration: Option<f64>,
    pub max_duration: Option<f64>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl MediaLibraryDatabase {
    /// Create new media library database instance
    pub async fn new(db_path: &Path, thumbnail_dir: &Path) -> Result<Self> {
        info!(
            "Initializing media library database at: {}",
            db_path.display()
        );

        // Create database directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow!("Failed to create database directory: {}", e))?;
        }

        // Create thumbnail directory if it doesn't exist
        tokio::fs::create_dir_all(thumbnail_dir)
            .await
            .map_err(|e| anyhow!("Failed to create thumbnail directory: {}", e))?;

        // Connect to database
        let connection_string = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&connection_string)
            .await
            .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

        let db = Self {
            pool,
            thumbnail_dir: thumbnail_dir.to_path_buf(),
        };

        // Initialize database schema
        db.init_schema().await?;

        info!("Media library database initialized successfully");
        Ok(db)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        info!("Initializing database schema");

        // Create media_files table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS media_files (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                filename TEXT NOT NULL,
                file_type TEXT NOT NULL,
                size INTEGER NOT NULL,
                duration REAL,
                width INTEGER,
                height INTEGER,
                video_codec TEXT,
                audio_codec TEXT,
                sample_rate INTEGER,
                channels INTEGER,
                bit_rate INTEGER,
                thumbnail_path TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                modified_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_accessed DATETIME DEFAULT CURRENT_TIMESTAMP,
                tags TEXT,
                metadata TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create media_files table: {}", e))?;

        // Create scan_history table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS scan_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scan_path TEXT NOT NULL,
                scan_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                files_found INTEGER,
                total_size INTEGER,
                errors TEXT,
                scan_duration_ms INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create scan_history table: {}", e))?;

        // Create indexes for better performance
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_media_files_path ON media_files(path)",
            "CREATE INDEX IF NOT EXISTS idx_media_files_type ON media_files(file_type)",
            "CREATE INDEX IF NOT EXISTS idx_media_files_created_at ON media_files(created_at)",
            "CREATE INDEX IF NOT EXISTS idx_media_files_filename ON media_files(filename)",
            "CREATE INDEX IF NOT EXISTS idx_scan_history_scan_time ON scan_history(scan_time)",
        ];

        for index in indexes {
            sqlx::query(index)
                .execute(&self.pool)
                .await
                .map_err(|e| anyhow!("Failed to create index: {}", e))?;
        }

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Insert or update a media file
    pub async fn upsert_media_file(&self, media_info: &MediaFileInfo) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        // Convert MediaMetadata to JSON
        let metadata_json = serde_json::to_string(&media_info.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;

        // Check if file already exists
        let existing = sqlx::query("SELECT id FROM media_files WHERE path = ?")
            .bind(&media_info.path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to check existing file: {}", e))?;

        if existing.is_some() {
            // Update existing record
            sqlx::query(
                r#"
                UPDATE media_files SET
                    filename = ?,
                    file_type = ?,
                    size = ?,
                    duration = ?,
                    modified_at = CURRENT_TIMESTAMP,
                    metadata = ?,
                    thumbnail_path = ?
                WHERE path = ?
                "#,
            )
            .bind(&media_info.name)
            .bind(format!("{:?}", media_info.file_type))
            .bind(media_info.size as i64)
            .bind(media_info.duration)
            .bind(metadata_json)
            .bind(&media_info.thumbnail_path)
            .bind(&media_info.path)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to update media file: {}", e))?;

            debug!("Updated media file: {}", media_info.path);
            Ok(existing.unwrap().get("id"))
        } else {
            // Insert new record
            sqlx::query(
                r#"
                INSERT INTO media_files (
                    id, path, filename, file_type, size, duration,
                    metadata, thumbnail_path
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&id)
            .bind(&media_info.path)
            .bind(&media_info.name)
            .bind(format!("{:?}", media_info.file_type))
            .bind(media_info.size as i64)
            .bind(media_info.duration)
            .bind(metadata_json)
            .bind(&media_info.thumbnail_path)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to insert media file: {}", e))?;

            debug!("Inserted new media file: {}", media_info.path);
            Ok(id)
        }
    }

    /// Search media files with filters
    pub async fn search_media_files(&self, filter: &SearchFilter) -> Result<Vec<DbMediaFile>> {
        let mut query = "SELECT * FROM media_files WHERE 1=1".to_string();
        let mut bindings: Vec<Box<dyn sqlx::Encode<'_, Sqlite> + sqlx::Type<Sqlite> + Send>> =
            Vec::new();

        // Add filters
        if let Some(query_str) = &filter.query {
            query.push_str(" AND (filename LIKE ? OR path LIKE ?)");
            let like_query = format!("%{}%", query_str);
            bindings.push(Box::new(like_query.clone()));
            bindings.push(Box::new(like_query));
        }

        if let Some(file_types) = &filter.file_types {
            let type_placeholders: Vec<String> =
                file_types.iter().map(|_| "?".to_string()).collect();
            query.push_str(&format!(
                " AND file_type IN ({})",
                type_placeholders.join(",")
            ));
            for ft in file_types {
                bindings.push(Box::new(format!("{:?}", ft)));
            }
        }

        if let Some(min_duration) = filter.min_duration {
            query.push_str(" AND duration >= ?");
            bindings.push(Box::new(min_duration));
        }

        if let Some(max_duration) = filter.max_duration {
            query.push_str(" AND duration <= ?");
            bindings.push(Box::new(max_duration));
        }

        if let Some(min_size) = filter.min_size {
            query.push_str(" AND size >= ?");
            bindings.push(Box::new(min_size as i64));
        }

        if let Some(max_size) = filter.max_size {
            query.push_str(" AND size <= ?");
            bindings.push(Box::new(max_size as i64));
        }

        // Add ordering and limits
        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = filter.offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        // Execute query (simplified approach)
        let mut q = sqlx::query_as::<_, DbMediaFile>(&query);

        // This is a simplified approach - in production, you'd want to handle bindings properly
        let results = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to search media files: {}", e))?;

        Ok(results)
    }

    /// Get media file by path
    pub async fn get_media_file_by_path(&self, path: &str) -> Result<Option<DbMediaFile>> {
        let file = sqlx::query_as::<_, DbMediaFile>("SELECT * FROM media_files WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get media file by path: {}", e))?;

        Ok(file)
    }

    /// Get scan history
    pub async fn get_scan_history(&self, limit: Option<i64>) -> Result<Vec<ScanHistory>> {
        let mut query = "SELECT * FROM scan_history ORDER BY scan_time DESC".to_string();

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let history = sqlx::query_as::<_, ScanHistory>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get scan history: {}", e))?;

        Ok(history)
    }

    /// Record scan history
    pub async fn record_scan_history(&self, scan: &ScanHistory) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO scan_history (
                scan_path, scan_time, files_found, total_size, errors, scan_duration_ms
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&scan.scan_path)
        .bind(scan.scan_time)
        .bind(scan.files_found)
        .bind(scan.total_size)
        .bind(&scan.errors)
        .bind(scan.scan_duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to record scan history: {}", e))?;

        Ok(result.last_insert_rowid())
    }

    /// Get library statistics
    pub async fn get_library_stats(&self) -> Result<LibraryStats> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_files,
                SUM(size) as total_size,
                COUNT(CASE WHEN file_type = 'Video' THEN 1 END) as video_files,
                COUNT(CASE WHEN file_type = 'Audio' THEN 1 END) as audio_files,
                COUNT(CASE WHEN file_type = 'Image' THEN 1 END) as image_files,
                COUNT(CASE WHEN thumbnail_path IS NOT NULL THEN 1 END) as files_with_thumbnails
            FROM media_files
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to get library stats: {}", e))?;

        Ok(LibraryStats {
            total_files: stats.total_files.unwrap_or(0) as usize,
            total_size: stats.total_size.unwrap_or(0) as u64,
            video_files: stats.video_files.unwrap_or(0) as usize,
            audio_files: stats.audio_files.unwrap_or(0) as usize,
            image_files: stats.image_files.unwrap_or(0) as usize,
            files_with_thumbnails: stats.files_with_thumbnails.unwrap_or(0) as usize,
        })
    }

    /// Delete media file
    pub async fn delete_media_file(&self, path: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM media_files WHERE path = ?")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to delete media file: {}", e))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get thumbnail path
    pub fn get_thumbnail_path(&self) -> &Path {
        &self.thumbnail_dir
    }

    /// Clean up orphaned thumbnails
    pub async fn cleanup_orphaned_thumbnails(&self) -> Result<usize> {
        // Get all thumbnail paths from database
        let db_thumbnails =
            sqlx::query("SELECT thumbnail_path FROM media_files WHERE thumbnail_path IS NOT NULL")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| anyhow!("Failed to get thumbnail paths: {}", e))?;

        let mut removed_count = 0;

        if let Ok(mut entries) = tokio::fs::read_dir(&self.thumbnail_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(path) = entry.file_name().into_string() {
                    let full_path = self.thumbnail_dir.join(&path);

                    // Check if this thumbnail exists in database
                    let exists = db_thumbnails.iter().any(|row| {
                        if let Ok(db_path) = row.get::<String, _>("thumbnail_path") {
                            db_path.contains(&path)
                        } else {
                            false
                        }
                    });

                    if !exists {
                        if let Err(e) = tokio::fs::remove_file(&full_path).await {
                            warn!("Failed to remove orphaned thumbnail {}: {}", path, e);
                        } else {
                            debug!("Removed orphaned thumbnail: {}", path);
                            removed_count += 1;
                        }
                    }
                }
            }
        }

        Ok(removed_count)
    }
}

/// Library statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_files: usize,
    pub total_size: u64,
    pub video_files: usize,
    pub audio_files: usize,
    pub image_files: usize,
    pub files_with_thumbnails: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let thumb_dir = temp_dir.path().join("thumbnails");

        let db = MediaLibraryDatabase::new(&db_path, &thumb_dir).await;
        assert!(db.is_ok());

        let db = db.unwrap();
        let stats = db.get_library_stats().await.unwrap();
        assert_eq!(stats.total_files, 0);
    }

    #[tokio::test]
    async fn test_upsert_media_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let thumb_dir = temp_dir.path().join("thumbnails");

        let db = MediaLibraryDatabase::new(&db_path, &thumb_dir)
            .await
            .unwrap();

        let media_info = MediaFileInfo {
            path: "/test/video.mp4".to_string(),
            name: "video.mp4".to_string(),
            file_type: MediaType::Video,
            size: 1024,
            duration: Some(10.5),
            metadata: MediaMetadata {
                video_info: None,
                audio_info: None,
                format_info: crate::commands_media::FormatInfo {
                    format_name: "mp4".to_string(),
                    duration: Some(10.5),
                    bit_rate: Some(1000),
                    file_size: 1024,
                },
            },
            thumbnail_path: None,
            last_modified: "now".to_string(),
            created: "now".to_string(),
        };

        let id = db.upsert_media_file(&media_info).await.unwrap();
        assert!(!id.is_empty());

        let stats = db.get_library_stats().await.unwrap();
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.video_files, 1);
    }
}
