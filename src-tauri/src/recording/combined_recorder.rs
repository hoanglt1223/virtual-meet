//! Combined Recording Pipeline
//!
//! This module provides real-time recording of current video + audio output to MP4 files
//! with configurable resolution (720p/1080p) and quality presets, including proper A/V sync.

use anyhow::{anyhow, Result};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex as StdMutex,
};
use std::thread;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, warn};

use crate::audio::{AudioFrameData, AudioSampleFormat};
use crate::devices::{DeviceOrigin, DeviceType, FullDeviceInfo};
use crate::recording::av_sync::AVSynchronizer;
use crate::recording::config::{
    AudioQualityPreset, RecordingConfig, VideoQualityPreset, VideoResolution,
};
use crate::recording::mp4_muxer::MP4Muxer;

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingState {
    Idle,
    Starting,
    Recording,
    Stopping,
    Error,
}

/// Combined audio/video frame with synchronized timestamps
#[derive(Debug, Clone)]
pub struct AVFrame {
    /// Frame timestamp
    pub timestamp: Duration,
    /// Video frame data (if available)
    pub video_frame: Option<VideoFrameData>,
    /// Audio frame data (if available)
    pub audio_frame: Option<AudioFrameData>,
    /// Frame sequence number
    pub sequence_number: u64,
}

/// Video frame data
#[derive(Debug, Clone)]
pub struct VideoFrameData {
    /// Raw frame data (RGB or YUV)
    pub data: Vec<u8>,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Frame format
    pub format: VideoFormat,
    /// Frame timestamp
    pub timestamp: Duration,
    /// Frame number
    pub frame_number: u64,
    /// Duration of this frame
    pub duration: Duration,
}

/// Video format enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoFormat {
    RGB24,
    YUV420P,
    NV12,
    YUY2,
}

impl VideoFormat {
    /// Get bytes per pixel for this format
    pub fn bytes_per_pixel(self) -> f32 {
        match self {
            VideoFormat::RGB24 => 3.0,
            VideoFormat::YUV420P => 1.5,
            VideoFormat::NV12 => 1.5,
            VideoFormat::YUY2 => 2.0,
        }
    }

    /// Get the name of this format
    pub fn name(self) -> &'static str {
        match self {
            VideoFormat::RGB24 => "rgb24",
            VideoFormat::YUV420P => "yuv420p",
            VideoFormat::NV12 => "nv12",
            VideoFormat::YUY2 => "yuy2",
        }
    }
}

/// Recording session configuration
#[derive(Debug, Clone)]
pub struct RecordingSession {
    /// Unique session ID
    pub id: String,
    /// Output file path
    pub output_path: PathBuf,
    /// Recording configuration
    pub config: RecordingConfig,
    /// Session start time
    pub start_time: Option<Instant>,
    /// Session duration
    pub duration: Duration,
    /// Number of recorded frames
    pub video_frames_recorded: u64,
    /// Number of recorded audio frames
    pub audio_frames_recorded: u64,
    /// Current file size in bytes
    pub file_size: u64,
}

impl RecordingSession {
    pub fn new(id: String, output_path: PathBuf, config: RecordingConfig) -> Self {
        Self {
            id,
            output_path,
            config,
            start_time: None,
            duration: Duration::ZERO,
            video_frames_recorded: 0,
            audio_frames_recorded: 0,
            file_size: 0,
        }
    }

    /// Check if session is currently recording
    pub fn is_recording(&self) -> bool {
        self.start_time.is_some()
    }

    /// Get session duration if recording
    pub fn get_duration(&self) -> Duration {
        if let Some(start_time) = self.start_time {
            start_time.elapsed()
        } else {
            Duration::ZERO
        }
    }

    /// Start recording
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.duration = Duration::ZERO;
        self.video_frames_recorded = 0;
        self.audio_frames_recorded = 0;
    }

    /// Stop recording
    pub fn stop(&mut self) {
        if let Some(start_time) = self.start_time {
            self.duration = start_time.elapsed();
        }
        self.start_time = None;
    }
}

/// Statistics about the recording process
#[derive(Debug, Clone)]
pub struct RecordingStats {
    /// Current recording state
    pub state: RecordingState,
    /// Current session information
    pub session: Option<RecordingSession>,
    /// Total recording time
    pub total_recording_time: Duration,
    /// Average video frame rate
    pub average_fps: f32,
    /// Average audio sample rate
    pub average_sample_rate: u32,
    /// Number of dropped video frames
    pub dropped_video_frames: u64,
    /// Number of dropped audio frames
    pub dropped_audio_frames: u64,
    /// Current memory usage in bytes
    pub memory_usage: u64,
    /// Disk space used in bytes
    pub disk_usage: u64,
}

/// Combined audio/video recorder with synchronization
pub struct CombinedRecorder {
    /// Current recording state
    state: Arc<StdMutex<RecordingState>>,
    /// Current recording session
    current_session: Arc<StdMutex<Option<RecordingSession>>>,
    /// Recording configuration
    config: Arc<StdMutex<RecordingConfig>>,
    /// AV synchronizer for timestamp alignment
    av_sync: Arc<StdMutex<AVSynchronizer>>,
    /// MP4 muxer for output file creation
    muxer: Arc<StdMutex<Option<MP4Muxer>>>,
    /// Frame sender for the recording thread
    frame_sender: Arc<StdMutex<Option<UnboundedSender<AVFrame>>>>,
    /// Recording thread handle
    recording_thread: Arc<StdMutex<Option<thread::JoinHandle<()>>>>,
    /// Statistics
    stats: Arc<StdMutex<RecordingStats>>,
    /// Shutdown flag
    shutdown_flag: Arc<AtomicBool>,
    /// Frame sequence counter
    frame_counter: Arc<AtomicU64>,
}

impl CombinedRecorder {
    /// Create a new combined recorder
    pub fn new(config: RecordingConfig) -> Result<Self> {
        let recorder = Self {
            state: Arc::new(StdMutex::new(RecordingState::Idle)),
            current_session: Arc::new(StdMutex::new(None)),
            config: Arc::new(StdMutex::new(config)),
            av_sync: Arc::new(StdMutex::new(AVSynchronizer::new())),
            muxer: Arc::new(StdMutex::new(None)),
            frame_sender: Arc::new(StdMutex::new(None)),
            recording_thread: Arc::new(StdMutex::new(None)),
            stats: Arc::new(StdMutex::new(RecordingStats {
                state: RecordingState::Idle,
                session: None,
                total_recording_time: Duration::ZERO,
                average_fps: 0.0,
                average_sample_rate: 0,
                dropped_video_frames: 0,
                dropped_audio_frames: 0,
                memory_usage: 0,
                disk_usage: 0,
            })),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            frame_counter: Arc::new(AtomicU64::new(0)),
        };

        // Start the background recording thread
        recorder.start_recording_thread()?;

        Ok(recorder)
    }

    /// Start recording to the specified output file
    pub fn start_recording<P: AsRef<Path>>(&mut self, output_path: P) -> Result<String> {
        let output_path = output_path.as_ref().to_path_buf();

        // Check state
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            if *state != RecordingState::Idle {
                return Err(anyhow!("Cannot start recording - not idle"));
            }
            *state = RecordingState::Starting;
        }

        // Create session ID
        let session_id = format!("recording_{}", chrono::Utc::now().timestamp_nanos());

        // Clone configuration
        let config = {
            let config_guard = self
                .config
                .lock()
                .map_err(|_| anyhow!("Failed to lock config"))?;
            config_guard.clone()
        };

        // Create recording session
        let mut session = RecordingSession::new(session_id.clone(), output_path, config);
        session.start();

        // Update current session
        {
            let mut current_session = self
                .current_session
                .lock()
                .map_err(|_| anyhow!("Failed to lock session"))?;
            *current_session = Some(session.clone());
        }

        // Initialize MP4 muxer
        {
            let mut muxer_guard = self
                .muxer
                .lock()
                .map_err(|_| anyhow!("Failed to lock muxer"))?;
            *muxer_guard = Some(MP4Muxer::new(&session.output_path, &session.config)?);
        }

        // Reset AV synchronizer
        {
            let mut av_sync = self
                .av_sync
                .lock()
                .map_err(|_| anyhow!("Failed to lock AV synchronizer"))?;
            av_sync.reset();
        }

        // Update statistics
        {
            let mut stats = self
                .stats
                .lock()
                .map_err(|_| anyhow!("Failed to lock stats"))?;
            stats.state = RecordingState::Recording;
            stats.session = Some(session.clone());
        }

        // Update state
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            *state = RecordingState::Recording;
        }

        info!(
            "Started recording session {} to {}",
            session_id,
            session.output_path.display()
        );
        Ok(session_id)
    }

    /// Stop the current recording
    pub fn stop_recording(&mut self) -> Result<()> {
        // Check state
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            if *state != RecordingState::Recording {
                return Err(anyhow!("Cannot stop recording - not recording"));
            }
            *state = RecordingState::Stopping;
        }

        // Stop session
        let session = {
            let mut current_session = self
                .current_session
                .lock()
                .map_err(|_| anyhow!("Failed to lock session"))?;
            if let Some(ref mut session) = *current_session {
                session.stop();
                Some(session.clone())
            } else {
                None
            }
        };

        // Finalize MP4 muxer
        {
            let mut muxer_guard = self
                .muxer
                .lock()
                .map_err(|_| anyhow!("Failed to lock muxer"))?;
            if let Some(ref mut muxer) = *muxer_guard {
                muxer.finalize()?;
                info!("Finalized MP4 recording");
            }
            *muxer_guard = None;
        }

        // Update statistics
        {
            let mut stats = self
                .stats
                .lock()
                .map_err(|_| anyhow!("Failed to lock stats"))?;
            stats.state = RecordingState::Idle;
            if let Some(ref session) = session {
                stats.total_recording_time += session.duration;
            }
        }

        // Update state
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            *state = RecordingState::Idle;
        }

        if let Some(session) = session {
            info!(
                "Stopped recording session {}, duration: {:?}",
                session.id, session.duration
            );
        }

        Ok(())
    }

    /// Submit a video frame for recording
    pub fn submit_video_frame(&mut self, frame: VideoFrameData) -> Result<()> {
        // Check if we're recording
        {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            if *state != RecordingState::Recording {
                return Ok(()); // Silently ignore frames when not recording
            }
        }

        // Create AV frame
        let av_frame = AVFrame {
            timestamp: frame.timestamp,
            video_frame: Some(frame),
            audio_frame: None,
            sequence_number: self.frame_counter.fetch_add(1, Ordering::SeqCst),
        };

        // Send to recording thread
        {
            let sender_guard = self
                .frame_sender
                .lock()
                .map_err(|_| anyhow!("Failed to lock frame sender"))?;
            if let Some(ref sender) = *sender_guard {
                sender
                    .send(av_frame)
                    .map_err(|_| anyhow!("Failed to send video frame"))?;
            }
        }

        Ok(())
    }

    /// Submit an audio frame for recording
    pub fn submit_audio_frame(&mut self, frame: AudioFrameData) -> Result<()> {
        // Check if we're recording
        {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            if *state != RecordingState::Recording {
                return Ok(()); // Silently ignore frames when not recording
            }
        }

        // Create AV frame
        let av_frame = AVFrame {
            timestamp: frame.timestamp,
            video_frame: None,
            audio_frame: Some(frame),
            sequence_number: self.frame_counter.fetch_add(1, Ordering::SeqCst),
        };

        // Send to recording thread
        {
            let sender_guard = self
                .frame_sender
                .lock()
                .map_err(|_| anyhow!("Failed to lock frame sender"))?;
            if let Some(ref sender) = *sender_guard {
                sender
                    .send(av_frame)
                    .map_err(|_| anyhow!("Failed to send audio frame"))?;
            }
        }

        Ok(())
    }

    /// Update recording configuration
    pub fn update_config(&mut self, config: RecordingConfig) -> Result<()> {
        // Only allow config changes when not recording
        {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording state"))?;
            if *state == RecordingState::Recording {
                return Err(anyhow!("Cannot update configuration while recording"));
            }
        }

        let mut config_guard = self
            .config
            .lock()
            .map_err(|_| anyhow!("Failed to lock config"))?;
        *config_guard = config;

        info!("Updated recording configuration");
        Ok(())
    }

    /// Get current recording statistics
    pub fn get_stats(&self) -> Result<RecordingStats> {
        let stats = self
            .stats
            .lock()
            .map_err(|_| anyhow!("Failed to lock stats"))?;
        Ok(stats.clone())
    }

    /// Get current recording state
    pub fn get_state(&self) -> Result<RecordingState> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock recording state"))?;
        Ok(*state)
    }

    /// Get current session information
    pub fn get_current_session(&self) -> Result<Option<RecordingSession>> {
        let session = self
            .current_session
            .lock()
            .map_err(|_| anyhow!("Failed to lock session"))?;
        Ok(session.clone())
    }

    /// Start the background recording thread
    fn start_recording_thread(&self) -> Result<()> {
        let (frame_tx, frame_rx) = mpsc::unbounded_channel();

        // Store the sender
        {
            let mut sender_guard = self
                .frame_sender
                .lock()
                .map_err(|_| anyhow!("Failed to lock frame sender"))?;
            *sender_guard = Some(frame_tx);
        }

        // Clone references for the thread
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let av_sync = Arc::clone(&self.av_sync);
        let muxer = Arc::clone(&self.muxer);
        let current_session = Arc::clone(&self.current_session);
        let stats = Arc::clone(&self.stats);

        // Start the thread
        let thread_handle = thread::spawn(move || {
            Self::recording_thread_main(
                frame_rx,
                shutdown_flag,
                av_sync,
                muxer,
                current_session,
                stats,
            );
        });

        // Store the thread handle
        {
            let mut thread_guard = self
                .recording_thread
                .lock()
                .map_err(|_| anyhow!("Failed to lock recording thread"))?;
            *thread_guard = Some(thread_handle);
        }

        info!("Started recording thread");
        Ok(())
    }

    /// Main recording thread function
    fn recording_thread_main(
        mut frame_rx: UnboundedReceiver<AVFrame>,
        shutdown_flag: Arc<AtomicBool>,
        av_sync: Arc<StdMutex<AVSynchronizer>>,
        muxer: Arc<StdMutex<Option<MP4Muxer>>>,
        current_session: Arc<StdMutex<Option<RecordingSession>>>,
        stats: Arc<StdMutex<RecordingStats>>,
    ) {
        info!("Recording thread started");

        loop {
            // Check for shutdown
            if shutdown_flag.load(Ordering::SeqCst) {
                info!("Recording thread shutting down");
                break;
            }

            // Receive frame with timeout
            match frame_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(av_frame) => {
                    // Process the frame
                    if let Err(e) =
                        Self::process_frame(av_frame, &av_sync, &muxer, &current_session, &stats)
                    {
                        error!("Error processing frame: {}", e);
                    }
                }
                Err(mpsc::error::RecvTimeoutError::Timeout) => {
                    // Timeout is normal - continue loop
                    continue;
                }
                Err(mpsc::error::RecvTimeoutError::Disconnected) => {
                    info!("Frame channel disconnected - shutting down recording thread");
                    break;
                }
            }
        }

        info!("Recording thread stopped");
    }

    /// Process a single frame
    fn process_frame(
        av_frame: AVFrame,
        av_sync: &Arc<StdMutex<AVSynchronizer>>,
        muxer: &Arc<StdMutex<Option<MP4Muxer>>>,
        current_session: &Arc<StdMutex<Option<RecordingSession>>>,
        stats: &Arc<StdMutex<RecordingStats>>,
    ) -> Result<()> {
        // Synchronize the frame
        let synced_frame = {
            let mut sync_guard = av_sync
                .lock()
                .map_err(|_| anyhow!("Failed to lock AV synchronizer"))?;
            sync_guard.synchronize_frame(av_frame)?
        };

        // Get muxer
        let mut muxer_guard = muxer.lock().map_err(|_| anyhow!("Failed to lock muxer"))?;
        let muxer = muxer_guard
            .as_mut()
            .ok_or_else(|| anyhow!("No muxer available"))?;

        // Process based on frame type
        if let Some(video_frame) = synced_frame.video_frame {
            muxer.write_video_frame(&video_frame)?;

            // Update stats
            if let Ok(mut stats_guard) = stats.lock() {
                stats_guard.video_frames_recorded += 1;
            }

            // Update session
            if let Ok(mut session_guard) = current_session.lock() {
                if let Some(ref mut session) = *session_guard {
                    session.video_frames_recorded += 1;
                }
            }

            debug!("Processed video frame at {:?}", synced_frame.timestamp);
        }

        if let Some(audio_frame) = synced_frame.audio_frame {
            muxer.write_audio_frame(&audio_frame)?;

            // Update stats
            if let Ok(mut stats_guard) = stats.lock() {
                stats_guard.audio_frames_recorded += 1;
            }

            // Update session
            if let Ok(mut session_guard) = current_session.lock() {
                if let Some(ref mut session) = *session_guard {
                    session.audio_frames_recorded += 1;
                }
            }

            debug!("Processed audio frame at {:?}", synced_frame.timestamp);
        }

        Ok(())
    }
}

impl Drop for CombinedRecorder {
    fn drop(&mut self) {
        // Set shutdown flag
        self.shutdown_flag.store(true, Ordering::SeqCst);

        // Stop recording if active
        if let Ok(state) = self.get_state() {
            if state == RecordingState::Recording {
                let _ = self.stop_recording();
            }
        }

        // Join recording thread
        if let Ok(mut thread_guard) = self.recording_thread.lock() {
            if let Some(thread_handle) = thread_guard.take() {
                let _ = thread_handle.join();
            }
        }
    }
}

/// Utility functions for frame creation
pub mod utils {
    use super::*;
    use crate::audio::AudioSampleFormat;

    /// Create a video frame from raw RGB data
    pub fn create_video_frame_rgb(
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp: Duration,
        frame_number: u64,
    ) -> VideoFrameData {
        VideoFrameData {
            data,
            width,
            height,
            format: VideoFormat::RGB24,
            timestamp,
            frame_number,
            duration: Duration::from_millis(33), // ~30fps
        }
    }

    /// Create a video frame from raw YUV data
    pub fn create_video_frame_yuv(
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp: Duration,
        frame_number: u64,
    ) -> VideoFrameData {
        VideoFrameData {
            data,
            width,
            height,
            format: VideoFormat::YUV420P,
            timestamp,
            frame_number,
            duration: Duration::from_millis(33), // ~30fps
        }
    }

    /// Create an audio frame from raw samples
    pub fn create_audio_frame(
        data: Vec<u8>,
        channels: u32,
        sample_rate: u32,
        sample_format: AudioSampleFormat,
        timestamp: Duration,
        frame_number: u64,
    ) -> AudioFrameData {
        let duration = Duration::from_secs_f64(
            data.len() as f64
                / (channels as f64 * sample_rate as f64 * sample_format.bytes_per_sample() as f64),
        );

        AudioFrameData {
            data,
            channels,
            sample_rate,
            sample_format,
            timestamp,
            frame_number,
            duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_video_frame_creation() {
        let rgb_data = vec![255; 1920 * 1080 * 3]; // 1080p RGB frame
        let frame = create_video_frame_rgb(rgb_data, 1920, 1080, Duration::from_secs(0), 1);

        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.format, VideoFormat::RGB24);
        assert_eq!(frame.frame_number, 1);
    }

    #[test]
    fn test_audio_frame_creation() {
        let audio_data = vec![0; 1024 * 2 * 4]; // 1024 samples, 2 channels, f32
        let frame = create_audio_frame(
            audio_data,
            2,
            48000,
            AudioSampleFormat::F32,
            Duration::from_secs(0),
            1,
        );

        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.sample_format, AudioSampleFormat::F32);
        assert_eq!(frame.frame_number, 1);
    }

    #[test]
    fn test_recording_session() {
        let output_dir = tempdir().unwrap();
        let output_path = output_dir.path().join("test.mp4");
        let config = RecordingConfig::default();

        let mut session = RecordingSession::new("test-session".to_string(), output_path, config);

        assert_eq!(session.id, "test-session");
        assert!(!session.is_recording());
        assert_eq!(session.get_duration(), Duration::ZERO);

        session.start();
        assert!(session.is_recording());
        thread::sleep(Duration::from_millis(10));

        session.stop();
        assert!(!session.is_recording());
        assert!(session.get_duration() > Duration::ZERO);
    }

    #[test]
    fn test_video_format() {
        assert_eq!(VideoFormat::RGB24.name(), "rgb24");
        assert_eq!(VideoFormat::YUV420P.name(), "yuv420p");
        assert_eq!(VideoFormat::RGB24.bytes_per_pixel(), 3.0);
        assert_eq!(VideoFormat::YUY2.bytes_per_pixel(), 2.0);
    }

    #[test]
    fn test_recording_state_transitions() {
        let mut state = RecordingState::Idle;

        assert_eq!(state, RecordingState::Idle);

        state = RecordingState::Starting;
        assert_eq!(state, RecordingState::Starting);

        state = RecordingState::Recording;
        assert_eq!(state, RecordingState::Recording);

        state = RecordingState::Stopping;
        assert_eq!(state, RecordingState::Stopping);

        state = RecordingState::Idle;
        assert_eq!(state, RecordingState::Idle);
    }
}
