//! Media Management Module
//!
//! This module handles media file operations, library management, and media processing.
//! The actual media functionality is implemented in individual modules:
//! - media_library.rs: Media library management
//! - media_scanner.rs: File system scanning
//! - metadata_extractor.rs: Media file metadata extraction
//! - thumbnail_generator.rs: Video thumbnail generation

// Re-export media functionality from individual modules
pub use crate::media_library::*;
pub use crate::media_scanner::*;
pub use crate::metadata_extractor::*;
pub use crate::thumbnail_generator::*;
