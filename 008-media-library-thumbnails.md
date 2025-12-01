---
title: "Media Library with Thumbnails"
status: "todo"
priority: "medium"
tags: ["media-library", "thumbnails", "ui", "file-management"]
---

# Task: Media Library with Thumbnails

## Description
Create a media library system that scans folders, generates thumbnails for video files, and provides quick file selection interface.

## Acceptance Criteria
- [ ] Scan specified folders for media files (MP4, MP3, AVI, MOV)
- [ ] Generate thumbnails for video files (extract keyframes)
- [ ] Display metadata (duration, resolution, file size)
- [ ] Fast loading and scrolling even with 100+ files
- [ ] Search and filter functionality
- [ ] Grid and list view options
- [ ] Drag-and-drop file support
- [ ] Recent files and favorites functionality

## Implementation Details
### Library Structure
```rust
pub struct MediaLibrary {
    items: Vec<MediaItem>,
    thumbnail_cache: ThumbnailCache,
    file_watcher: FileWatcher,
    scan_directories: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub id: String,
    pub path: PathBuf,
    pub media_type: MediaType,
    pub metadata: MediaMetadata,
    pub thumbnail_path: Option<PathBuf>,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub last_accessed: SystemTime,
}

pub struct MediaMetadata {
    pub duration: Option<Duration>,
    pub resolution: Option<(u32, u32)>,
    pub file_size: u64,
    pub created: SystemTime,
    pub modified: SystemTime,
}
```

### Thumbnail Generation
- Extract keyframes from video files
- Cache thumbnails for fast loading
- Support multiple thumbnail sizes
- Background processing for new files

### UI Components
- Thumbnail grid view
- Detailed list view
- Search bar with filters
- Tag management system
- Folder navigation

### Performance Optimizations
- Lazy loading of thumbnails
- Background file scanning
- Efficient caching strategy
- Database for metadata storage

## Dependencies
- `ffmpeg-next`: Video thumbnail extraction
- `sqlx`: SQLite for metadata storage
- `notify`: File system watching
- `image`: Image processing
- `walkdir`: Directory scanning

## Testing Requirements
- Test with large media libraries (1000+ files)
- Thumbnail generation performance
- Memory usage optimization
- Database query performance

## Estimated Time: 8-10 hours