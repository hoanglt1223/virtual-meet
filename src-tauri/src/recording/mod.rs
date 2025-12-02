//! Recording Module
//!
//! Combined audio/video recording pipeline with configurable quality settings,
//! proper A/V synchronization, and MP4 output support.

pub mod av_sync;
pub mod combined_recorder;
pub mod config;
pub mod mp4_muxer;

pub use av_sync::{AVSynchronizer, SyncStatistics};
pub use combined_recorder::{
    utils as recorder_utils, AVFrame, CombinedRecorder, RecordingSession, RecordingState,
    RecordingStats, VideoFormat, VideoFrameData,
};
pub use config::{
    AdvancedSettings, AudioCodec, AudioQualityPreset, AudioSettings, FrameRate, OutputFormat,
    OutputSettings, PreviewPosition, RecordingConfig, ThreadPriority, VideoCodec,
    VideoQualityPreset, VideoResolution, VideoSettings,
};
pub use mp4_muxer::{MP4Muxer, MuxerStatistics};
