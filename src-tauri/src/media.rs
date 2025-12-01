//! Media Management Module
//!
//! This module handles media file operations, library management, and media processing.

pub mod metadata;
pub mod scanner;
pub mod validator;

pub use metadata::*;
pub use scanner::*;
pub use validator::*;

/// Media file information and operations