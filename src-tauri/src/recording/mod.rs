//! Recording Module
//!
//! Combined audio/video recording pipeline with configurable quality settings,
//! proper A/V synchronization, and MP4 output support.

pub mod combined_recorder;
pub mod config;
pub mod av_sync;
pub mod mp4_muxer;

pub use combined_recorder::{
    CombinedRecorder, AVFrame, VideoFrameData, VideoFormat,
    RecordingState, RecordingSession, RecordingStats,
    utils as recorder_utils,
};
pub use config::{
    RecordingConfig, VideoSettings, AudioSettings, OutputSettings, AdvancedSettings,
    VideoResolution, VideoQualityPreset, AudioQualityPreset,
    VideoCodec, AudioCodec, FrameRate, OutputFormat, ThreadPriority,
    PreviewPosition,
};
pub use av_sync::{AVSynchronizer, SyncStatistics};
pub use mp4_muxer::{MP4Muxer, MuxerStatistics};