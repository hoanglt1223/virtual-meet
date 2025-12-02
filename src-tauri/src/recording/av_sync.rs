//! Audio/Video Synchronization
//!
//! This module provides synchronization between audio and video streams,
//! ensuring proper timestamp alignment and maintaining audio-video sync
//! throughout the recording process.

use anyhow::{anyhow, Result};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use crate::audio::{AudioFrameData, AudioSampleFormat};
use crate::recording::combined_recorder::{AVFrame, VideoFrameData};

/// Audio/Video synchronizer for maintaining proper sync between streams
pub struct AVSynchronizer {
    /// Video frame buffer
    video_buffer: VecDeque<SynchronizedFrame<VideoFrameData>>,
    /// Audio frame buffer
    audio_buffer: VecDeque<SynchronizedFrame<AudioFrameData>>,
    /// Base timestamp for synchronization
    base_timestamp: Option<Instant>,
    /// Current video timestamp
    current_video_timestamp: Duration,
    /// Current audio timestamp
    current_audio_timestamp: Duration,
    /// Audio drift compensation
    audio_drift_compensation: f32,
    /// Video drift compensation
    video_drift_compensation: f32,
    /// Maximum buffer size
    max_buffer_size: usize,
    /// Target sync tolerance
    sync_tolerance: Duration,
    /// Total frames processed
    video_frames_processed: u64,
    audio_frames_processed: u64,
    /// Dropped frames counters
    dropped_video_frames: u64,
    dropped_audio_frames: u64,
}

/// A synchronized frame with adjusted timestamp
#[derive(Debug, Clone)]
struct SynchronizedFrame<T> {
    /// Original frame data
    data: T,
    /// Original timestamp
    original_timestamp: Duration,
    /// Synchronized timestamp
    synced_timestamp: Duration,
    /// Frame sequence number
    sequence_number: u64,
    /// Whether this frame has been processed
    processed: bool,
}

impl<T> SynchronizedFrame<T> {
    fn new(data: T, timestamp: Duration, sequence_number: u64) -> Self {
        Self {
            data,
            original_timestamp: timestamp,
            synced_timestamp: timestamp,
            sequence_number,
            processed: false,
        }
    }
}

/// Sync statistics for monitoring synchronization quality
#[derive(Debug, Clone)]
pub struct SyncStatistics {
    /// Average video-audio time difference
    pub avg_video_audio_diff: Duration,
    /// Maximum video-audio time difference
    pub max_video_audio_diff: Duration,
    /// Current video buffer size
    pub video_buffer_size: usize,
    /// Current audio buffer size
    pub audio_buffer_size: usize,
    /// Video frames processed
    pub video_frames_processed: u64,
    /// Audio frames processed
    pub audio_frames_processed: u64,
    /// Video frames dropped
    pub video_frames_dropped: u64,
    /// Audio frames dropped
    pub audio_frames_dropped: u64,
    /// Audio drift percentage
    pub audio_drift_percent: f32,
    /// Video drift percentage
    pub video_drift_percent: f32,
}

impl AVSynchronizer {
    /// Create a new AV synchronizer
    pub fn new() -> Self {
        Self {
            video_buffer: VecDeque::with_capacity(120), // 4 seconds at 30fps
            audio_buffer: VecDeque::with_capacity(1920), // 40ms of audio at 48kHz
            base_timestamp: None,
            current_video_timestamp: Duration::ZERO,
            current_audio_timestamp: Duration::ZERO,
            audio_drift_compensation: 1.0,
            video_drift_compensation: 1.0,
            max_buffer_size: 5000,
            sync_tolerance: Duration::from_millis(40), // 40ms tolerance
            video_frames_processed: 0,
            audio_frames_processed: 0,
            dropped_video_frames: 0,
            dropped_audio_frames: 0,
        }
    }

    /// Reset the synchronizer state
    pub fn reset(&mut self) {
        self.video_buffer.clear();
        self.audio_buffer.clear();
        self.base_timestamp = None;
        self.current_video_timestamp = Duration::ZERO;
        self.current_audio_timestamp = Duration::ZERO;
        self.audio_drift_compensation = 1.0;
        self.video_drift_compensation = 1.0;
        self.video_frames_processed = 0;
        self.audio_frames_processed = 0;
        self.dropped_video_frames = 0;
        self.dropped_audio_frames = 0;

        debug!("AV Synchronizer reset");
    }

    /// Synchronize an AV frame and return it for processing
    pub fn synchronize_frame(&mut self, av_frame: AVFrame) -> Result<AVFrame> {
        // Initialize base timestamp if needed
        if self.base_timestamp.is_none() {
            self.base_timestamp = Some(Instant::now());
            debug!("Initialized base timestamp for AV sync");
        }

        let base_timestamp = self.base_timestamp.unwrap();

        // Process frame based on type
        if av_frame.video_frame.is_some() {
            self.synchronize_video_frame(av_frame, base_timestamp)
        } else if av_frame.audio_frame.is_some() {
            self.synchronize_audio_frame(av_frame, base_timestamp)
        } else {
            Err(anyhow!("AV frame contains neither video nor audio data"))
        }
    }

    /// Synchronize a video frame
    fn synchronize_video_frame(
        &mut self,
        mut av_frame: AVFrame,
        base_timestamp: Instant,
    ) -> Result<AVFrame> {
        let video_frame = av_frame
            .video_frame
            .take()
            .ok_or_else(|| anyhow!("No video frame in AV frame"))?;

        // Adjust timestamp relative to base
        let adjusted_timestamp = if video_frame.timestamp > Duration::ZERO {
            video_frame.timestamp
        } else {
            base_timestamp.elapsed()
        };

        // Apply drift compensation
        let compensated_timestamp = Duration::from_secs_f64(
            adjusted_timestamp.as_secs_f64() * self.video_drift_compensation as f64,
        );

        // Update current video timestamp
        self.current_video_timestamp = compensated_timestamp;

        // Create synchronized frame
        let sync_frame =
            SynchronizedFrame::new(video_frame, adjusted_timestamp, self.video_frames_processed);

        // Add to buffer
        self.video_buffer.push_back(sync_frame);
        self.video_frames_processed += 1;

        // Check if buffer is full and drop frames if necessary
        if self.video_buffer.len() > self.max_buffer_size {
            warn!("Video buffer overflow, dropping oldest frame");
            self.video_buffer.pop_front();
            self.dropped_video_frames += 1;
        }

        // Try to find the best frame to output
        if let Some(best_frame) = self.find_best_video_frame() {
            av_frame.video_frame = Some(best_frame.data);
            av_frame.timestamp = best_frame.synced_timestamp;
            Ok(av_frame)
        } else {
            // No suitable frame yet, return original
            av_frame.video_frame = Some(video_frame);
            av_frame.timestamp = compensated_timestamp;
            Ok(av_frame)
        }
    }

    /// Synchronize an audio frame
    fn synchronize_audio_frame(
        &mut self,
        mut av_frame: AVFrame,
        base_timestamp: Instant,
    ) -> Result<AVFrame> {
        let audio_frame = av_frame
            .audio_frame
            .take()
            .ok_or_else(|| anyhow!("No audio frame in AV frame"))?;

        // Adjust timestamp relative to base
        let adjusted_timestamp = if audio_frame.timestamp > Duration::ZERO {
            audio_frame.timestamp
        } else {
            base_timestamp.elapsed()
        };

        // Apply drift compensation
        let compensated_timestamp = Duration::from_secs_f64(
            adjusted_timestamp.as_secs_f64() * self.audio_drift_compensation as f64,
        );

        // Update current audio timestamp
        self.current_audio_timestamp = compensated_timestamp;

        // Create synchronized frame
        let sync_frame =
            SynchronizedFrame::new(audio_frame, adjusted_timestamp, self.audio_frames_processed);

        // Add to buffer
        self.audio_buffer.push_back(sync_frame);
        self.audio_frames_processed += 1;

        // Check if buffer is full and drop frames if necessary
        if self.audio_buffer.len() > self.max_buffer_size {
            warn!("Audio buffer overflow, dropping oldest frame");
            self.audio_buffer.pop_front();
            self.dropped_audio_frames += 1;
        }

        // Try to find the best frame to output
        if let Some(best_frame) = self.find_best_audio_frame() {
            av_frame.audio_frame = Some(best_frame.data);
            av_frame.timestamp = best_frame.synced_timestamp;
            Ok(av_frame)
        } else {
            // No suitable frame yet, return original
            av_frame.audio_frame = Some(audio_frame);
            av_frame.timestamp = compensated_timestamp;
            Ok(av_frame)
        }
    }

    /// Find the best video frame based on sync criteria
    fn find_best_video_frame(&mut self) -> Option<SynchronizedFrame<VideoFrameData>> {
        if self.audio_buffer.is_empty() {
            // No audio reference, return oldest video frame
            return self.video_buffer.front().cloned();
        }

        // Get current audio reference time
        let audio_ref_time = self.current_audio_timestamp;

        // Find video frame closest to audio reference
        let mut best_frame = None;
        let mut min_diff = Duration::MAX;

        for frame in &self.video_buffer {
            let diff = if frame.synced_timestamp > audio_ref_time {
                frame.synced_timestamp - audio_ref_time
            } else {
                audio_ref_time - frame.synced_timestamp
            };

            if diff < min_diff {
                min_diff = diff;
                best_frame = Some(frame.clone());
            }
        }

        // Remove the selected frame from buffer
        if let Some(ref frame) = best_frame {
            self.video_buffer
                .retain(|f| f.sequence_number != frame.sequence_number);
        }

        best_frame
    }

    /// Find the best audio frame based on sync criteria
    fn find_best_audio_frame(&mut self) -> Option<SynchronizedFrame<AudioFrameData>> {
        if self.video_buffer.is_empty() {
            // No video reference, return oldest audio frame
            return self.audio_buffer.front().cloned();
        }

        // Get current video reference time
        let video_ref_time = self.current_video_timestamp;

        // Find audio frame closest to video reference
        let mut best_frame = None;
        let mut min_diff = Duration::MAX;

        for frame in &self.audio_buffer {
            let diff = if frame.synced_timestamp > video_ref_time {
                frame.synced_timestamp - video_ref_time
            } else {
                video_ref_time - frame.synced_timestamp
            };

            if diff < min_diff {
                min_diff = diff;
                best_frame = Some(frame.clone());
            }
        }

        // Remove the selected frame from buffer
        if let Some(ref frame) = best_frame {
            self.audio_buffer
                .retain(|f| f.sequence_number != frame.sequence_number);
        }

        best_frame
    }

    /// Update drift compensation based on current sync status
    pub fn update_drift_compensation(&mut self) {
        if self.video_buffer.is_empty() || self.audio_buffer.is_empty() {
            return;
        }

        // Calculate time difference between streams
        let video_time = self.current_video_timestamp;
        let audio_time = self.current_audio_timestamp;

        if video_time > Duration::ZERO && audio_time > Duration::ZERO {
            let time_diff = if video_time > audio_time {
                video_time - audio_time
            } else {
                audio_time - video_time
            };

            // If difference exceeds tolerance, adjust compensation
            if time_diff > self.sync_tolerance {
                let adjustment_factor = 1.0 + (time_diff.as_secs_f64() / 10.0) as f32; // Gradual adjustment

                if video_time > audio_time {
                    // Video is ahead, slow it down slightly
                    self.video_drift_compensation = (1.0 / adjustment_factor).max(0.95);
                    debug!(
                        "Adjusting video drift compensation to {:.4}",
                        self.video_drift_compensation
                    );
                } else {
                    // Audio is ahead, slow it down slightly
                    self.audio_drift_compensation = (1.0 / adjustment_factor).max(0.95);
                    debug!(
                        "Adjusting audio drift compensation to {:.4}",
                        self.audio_drift_compensation
                    );
                }
            } else {
                // Within tolerance, gradually return to normal
                self.video_drift_compensation = self.video_drift_compensation * 0.99 + 1.0 * 0.01;
                self.audio_drift_compensation = self.audio_drift_compensation * 0.99 + 1.0 * 0.01;
            }
        }
    }

    /// Get current synchronization statistics
    pub fn get_statistics(&self) -> SyncStatistics {
        // Calculate average time difference
        let avg_diff = if !self.video_buffer.is_empty() && !self.audio_buffer.is_empty() {
            let video_time = self.current_video_timestamp;
            let audio_time = self.current_audio_timestamp;
            if video_time > audio_time {
                video_time - audio_time
            } else {
                audio_time - video_time
            }
        } else {
            Duration::ZERO
        };

        SyncStatistics {
            avg_video_audio_diff: avg_diff,
            max_video_audio_diff: self.sync_tolerance,
            video_buffer_size: self.video_buffer.len(),
            audio_buffer_size: self.audio_buffer.len(),
            video_frames_processed: self.video_frames_processed,
            audio_frames_processed: self.audio_frames_processed,
            video_frames_dropped: self.dropped_video_frames,
            audio_frames_dropped: self.dropped_audio_frames,
            audio_drift_percent: (self.audio_drift_compensation - 1.0) * 100.0,
            video_drift_percent: (self.video_drift_compensation - 1.0) * 100.0,
        }
    }

    /// Set synchronization tolerance
    pub fn set_sync_tolerance(&mut self, tolerance: Duration) {
        self.sync_tolerance = tolerance;
        debug!("Set sync tolerance to {:?}", tolerance);
    }

    /// Set maximum buffer size
    pub fn set_max_buffer_size(&mut self, size: usize) {
        self.max_buffer_size = size;
        debug!("Set max buffer size to {}", size);
    }

    /// Force alignment of streams (emergency sync)
    pub fn force_alignment(&mut self) {
        if !self.video_buffer.is_empty() && !self.audio_buffer.is_empty() {
            // Take the most recent frames and align them
            let latest_video = self.video_buffer.back().cloned();
            let latest_audio = self.audio_buffer.back().cloned();

            if let (Some(video), Some(audio)) = (latest_video, latest_audio) {
                let avg_timestamp = (video.synced_timestamp + audio.synced_timestamp) / 2;

                // Update current timestamps to the average
                self.current_video_timestamp = avg_timestamp;
                self.current_audio_timestamp = avg_timestamp;

                // Clear buffers to start fresh
                self.video_buffer.clear();
                self.audio_buffer.clear();

                warn!("Forced alignment of audio/video streams");
            }
        }
    }

    /// Get current sync quality score (0.0 to 1.0)
    pub fn get_sync_quality_score(&self) -> f32 {
        let stats = self.get_statistics();

        // Factors affecting sync quality
        let buffer_health = 1.0
            - ((stats.video_buffer_size + stats.audio_buffer_size) as f32
                / (self.max_buffer_size as f32 * 2.0));
        let time_diff_factor =
            1.0 - (stats.avg_video_audio_diff.as_secs_f32() / self.sync_tolerance.as_secs_f32());
        let drift_factor =
            1.0 - (stats.audio_drift_percent.abs() + stats.video_drift_percent.abs()) / 200.0;
        let dropout_factor = if stats.video_frames_processed > 0 && stats.audio_frames_processed > 0
        {
            1.0 - ((stats.video_frames_dropped + stats.audio_frames_dropped) as f32
                / (stats.video_frames_processed + stats.audio_frames_processed) as f32)
        } else {
            1.0
        };

        // Combine factors (weighted average)
        (buffer_health * 0.3 + time_diff_factor * 0.3 + drift_factor * 0.2 + dropout_factor * 0.2)
            .max(0.0)
            .min(1.0)
    }
}

impl Default for AVSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::combined_recorder::utils::*;
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_av_synchronizer_creation() {
        let sync = AVSynchronizer::new();
        assert_eq!(sync.video_buffer.len(), 0);
        assert_eq!(sync.audio_buffer.len(), 0);
        assert_eq!(sync.audio_drift_compensation, 1.0);
        assert_eq!(sync.video_drift_compensation, 1.0);
    }

    #[test]
    fn test_synchronizer_reset() {
        let mut sync = AVSynchronizer::new();
        sync.video_frames_processed = 100;
        sync.audio_frames_processed = 200;

        sync.reset();
        assert_eq!(sync.video_frames_processed, 0);
        assert_eq!(sync.audio_frames_processed, 0);
        assert!(sync.video_buffer.is_empty());
        assert!(sync.audio_buffer.is_empty());
    }

    #[test]
    fn test_drift_compensation() {
        let mut sync = AVSynchronizer::new();
        sync.audio_drift_compensation = 0.95;
        sync.video_drift_compensation = 1.05;

        sync.update_drift_compensation();

        // Should gradually return to 1.0
        assert!(sync.audio_drift_compensation > 0.95);
        assert!(sync.video_drift_compensation < 1.05);
    }

    #[test]
    fn test_sync_quality_score() {
        let sync = AVSynchronizer::new();
        let score = sync.get_sync_quality_score();
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_synchronized_frame_creation() {
        let data = vec![1, 2, 3, 4];
        let timestamp = Duration::from_millis(100);
        let frame = SynchronizedFrame::new(data, timestamp, 1);

        assert_eq!(frame.original_timestamp, timestamp);
        assert_eq!(frame.sequence_number, 1);
        assert!(!frame.processed);
    }

    #[test]
    fn test_sync_statistics() {
        let mut sync = AVSynchronizer::new();
        sync.video_frames_processed = 100;
        sync.audio_frames_processed = 150;
        sync.dropped_video_frames = 5;
        sync.dropped_audio_frames = 3;

        let stats = sync.get_statistics();
        assert_eq!(stats.video_frames_processed, 100);
        assert_eq!(stats.audio_frames_processed, 150);
        assert_eq!(stats.video_frames_dropped, 5);
        assert_eq!(stats.audio_frames_dropped, 3);
    }
}
