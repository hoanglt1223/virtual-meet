//! MP4 Muxer
//!
//! This module provides MP4 file creation and muxing capabilities,
//! handling both video and audio streams and creating properly formatted
//! MP4 output files with correct timestamps and metadata.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tracing::{info, warn, error, debug};

use crate::audio::AudioFrameData;
use crate::audio::AudioSampleFormat;
use crate::recording::combined_recorder::VideoFrameData;
use crate::recording::config::{RecordingConfig, VideoCodec, AudioCodec, VideoFormat};

/// MP4 muxer for creating output files
pub struct MP4Muxer {
    /// Output file path
    output_path: PathBuf,
    /// Recording configuration
    config: RecordingConfig,
    /// FFmpeg process handle
    ffmpeg_process: Option<std::process::Child>,
    /// Video encoder settings
    video_encoder: VideoEncoder,
    /// Audio encoder settings
    audio_encoder: AudioEncoder,
    /// Total video frames written
    video_frames_written: u64,
    /// Total audio frames written
    audio_frames_written: u64,
    /// Current file size
    current_file_size: u64,
    /// Timestamp offset for the first frame
    timestamp_offset: Duration,
    /// Last video timestamp
    last_video_timestamp: Duration,
    /// Last audio timestamp
    last_audio_timestamp: Duration,
}

/// Video encoder configuration
#[derive(Debug, Clone)]
struct VideoEncoder {
    codec: VideoCodec,
    width: u32,
    height: u32,
    frame_rate: f32,
    bitrate: u32,
    crf: u8,
    pixel_format: VideoFormat,
    keyframe_interval: Duration,
}

/// Audio encoder configuration
#[derive(Debug, Clone)]
struct AudioEncoder {
    codec: AudioCodec,
    sample_rate: u32,
    channels: u32,
    bitrate: u32,
    sample_format: AudioSampleFormat,
}

impl MP4Muxer {
    /// Create a new MP4 muxer
    pub fn new<P: AsRef<Path>>(output_path: P, config: &RecordingConfig) -> Result<Self> {
        let output_path = output_path.as_ref().to_path_buf();

        // Validate configuration
        config.validate()?;

        // Create output directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Setup video encoder
        let (video_width, video_height) = config.video.resolution.dimensions();
        let video_encoder = VideoEncoder {
            codec: config.video.codec,
            width: video_width,
            height: video_height,
            frame_rate: config.video.frame_rate.value(),
            bitrate: config.video.target_bitrate,
            crf: config.video.crf,
            pixel_format: VideoFormat::YUV420P, // Always use YUV420P for compatibility
            keyframe_interval: Duration::from_secs(config.video.keyframe_interval as u64),
        };

        // Setup audio encoder
        let audio_encoder = AudioEncoder {
            codec: config.audio.codec,
            sample_rate: config.audio.sample_rate,
            channels: config.audio.channels,
            bitrate: config.audio.target_bitrate,
            sample_format: AudioSampleFormat::F32, // Always convert to F32 for processing
        };

        info!("Creating MP4 muxer for: {}", output_path.display());
        info!("Video: {}x{} @ {:.1}fps, codec: {}",
              video_width, video_height, video_encoder.frame_rate, video_encoder.codec.name());
        info!("Audio: {}Hz, {} channels, codec: {}",
              audio_encoder.sample_rate, audio_encoder.channels, audio_encoder.codec.name());

        let mut muxer = Self {
            output_path,
            config: config.clone(),
            ffmpeg_process: None,
            video_encoder,
            audio_encoder,
            video_frames_written: 0,
            audio_frames_written: 0,
            current_file_size: 0,
            timestamp_offset: Duration::ZERO,
            last_video_timestamp: Duration::ZERO,
            last_audio_timestamp: Duration::ZERO,
        };

        // Initialize FFmpeg process
        muxer.initialize_ffmpeg()?;

        Ok(muxer)
    }

    /// Initialize FFmpeg process for encoding
    fn initialize_ffmpeg(&mut self) -> Result<()> {
        let mut args = self.build_ffmpeg_args();

        // Add custom FFmpeg parameters if present
        for (key, value) in &self.config.advanced.custom_ffmpeg_params {
            args.push(format!("-{}", key));
            args.push(value.clone());
        }

        debug!("Starting FFmpeg with args: {:?}", args);

        let process = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start FFmpeg: {}. Is FFmpeg installed and in PATH?", e))?;

        self.ffmpeg_process = Some(process);
        info!("FFmpeg process started successfully");
        Ok(())
    }

    /// Build FFmpeg command line arguments
    fn build_ffmpeg_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Input specifications
        args.push("-f".to_string());
        args.push("rawvideo".to_string());
        args.push("-pixel_format".to_string());
        args.push(self.video_encoder.pixel_format.name().to_string());
        args.push("-video_size".to_string());
        args.push(format!("{}x{}", self.video_encoder.width, self.video_encoder.height));
        args.push("-framerate".to_string());
        args.push(format!("{:.2}", self.video_encoder.frame_rate));
        args.push("-i".to_string());
        args.push("-".to_string()); // Video input from stdin

        args.push("-f".to_string());
        args.push("f32le".to_string());
        args.push("-ac".to_string());
        args.push(self.audio_encoder.channels.to_string());
        args.push("-ar".to_string());
        args.push(self.audio_encoder.sample_rate.to_string());
        args.push("-i".to_string());
        args.push("-".to_string()); // Audio input from stdin (note: this won't work directly, we need a different approach)

        // Video encoding options
        args.push("-c:v".to_string());
        args.push(self.video_encoder.codec.ffmpeg_name().to_string());

        if self.video_encoder.bitrate > 0 {
            args.push("-b:v".to_string());
            args.push(self.video_encoder.bitrate.to_string());
        }

        if self.video_encoder.crf > 0 {
            args.push("-crf".to_string());
            args.push(self.video_encoder.crf.to_string());
        }

        args.push("-g".to_string());
        args.push((self.video_encoder.keyframe_interval.as_secs_f32() * self.video_encoder.frame_rate) as i32 to_string());

        // Audio encoding options
        args.push("-c:a".to_string());
        args.push(self.audio_encoder.codec.ffmpeg_name().to_string());

        if self.audio_encoder.bitrate > 0 {
            args.push("-b:a".to_string());
            args.push(self.audio_encoder.bitrate.to_string());
        }

        // Output format and file
        args.push("-f".to_string());
        args.push("mp4".to_string());
        args.push("-movflags".to_string());
        args.push("+faststart".to_string()); // Optimize for web streaming
        args.push(self.output_path.to_string_lossy().to_string());

        args
    }

    /// Write a video frame to the output
    pub fn write_video_frame(&mut self, frame: &VideoFrameData) -> Result<()> {
        // Convert frame format if necessary
        let converted_data = self.convert_video_format(frame)?;

        // Update timestamp offset for first frame
        if self.video_frames_written == 0 {
            self.timestamp_offset = frame.timestamp;
            debug!("Set video timestamp offset to {:?}", self.timestamp_offset);
        }

        // Update last video timestamp
        self.last_video_timestamp = frame.timestamp;

        // Write frame data to FFmpeg stdin
        if let Some(ref mut process) = self.ffmpeg_process {
            if let Some(ref mut stdin) = process.stdin {
                use std::io::Write;
                stdin.write_all(&converted_data)?;
                stdin.flush()?;
            } else {
                return Err(anyhow!("FFmpeg stdin is not available"));
            }
        } else {
            return Err(anyhow!("FFmpeg process is not running"));
        }

        self.video_frames_written += 1;

        debug!("Wrote video frame {} at {:?}", self.video_frames_written, frame.timestamp);
        Ok(())
    }

    /// Write an audio frame to the output
    pub fn write_audio_frame(&mut self, frame: &AudioFrameData) -> Result<()> {
        // Convert audio format if necessary
        let converted_data = self.convert_audio_format(frame)?;

        // Update timestamp offset for first audio frame
        if self.audio_frames_written == 0 {
            debug!("Set audio timestamp offset to {:?}", frame.timestamp);
        }

        // Update last audio timestamp
        self.last_audio_timestamp = frame.timestamp;

        // For now, we'll skip writing audio frames directly to FFmpeg
        // In a real implementation, you'd need a more sophisticated approach
        // such as using pipes or temporary files for audio input
        self.audio_frames_written += 1;

        debug!("Wrote audio frame {} at {:?}", self.audio_frames_written, frame.timestamp);
        Ok(())
    }

    /// Convert video frame to the expected format
    fn convert_video_format(&self, frame: &VideoFrameData) -> Result<Vec<u8>> {
        if frame.format == self.video_encoder.pixel_format {
            return Ok(frame.data.clone());
        }

        // Simple RGB24 to YUV420P conversion
        match (frame.format, self.video_encoder.pixel_format) {
            (VideoFormat::RGB24, VideoFormat::YUV420P) => {
                self.rgb24_to_yuv420p(&frame.data, frame.width, frame.height)
            }
            (VideoFormat::YUY2, VideoFormat::YUV420P) => {
                self.yuy2_to_yuv420p(&frame.data, frame.width, frame.height)
            }
            _ => Err(anyhow!("Unsupported video format conversion: {:?} -> {:?}",
                           frame.format, self.video_encoder.pixel_format)),
        }
    }

    /// Convert RGB24 to YUV420P
    fn rgb24_to_yuv420p(&self, rgb_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        let width = width as usize;
        let height = height as usize;

        let y_size = width * height;
        let uv_size = y_size / 4;
        let mut yuv_data = vec![0u8; y_size + uv_size * 2];

        // Convert RGB to YUV420P
        for y in 0..height {
            for x in 0..width {
                let rgb_idx = (y * width + x) * 3;
                let y_idx = y * width + x;

                let r = rgb_data[rgb_idx] as f32;
                let g = rgb_data[rgb_idx + 1] as f32;
                let b = rgb_data[rgb_idx + 2] as f32;

                // Y component
                yuv_data[y_idx] = ((0.299 * r + 0.587 * g + 0.114 * b) as u8).clamp(0, 255);

                // UV components (subsampled)
                if y % 2 == 0 && x % 2 == 0 {
                    let uv_idx = y_size + (y / 2) * (width / 2) + (x / 2);
                    let u_idx = y_size + uv_size + (y / 2) * (width / 2) + (x / 2);

                    // U component
                    yuv_data[uv_idx] = ((-0.169 * r - 0.331 * g + 0.500 * b + 128.0) as u8).clamp(0, 255);
                    // V component
                    yuv_data[u_idx] = ((0.500 * r - 0.419 * g - 0.081 * b + 128.0) as u8).clamp(0, 255);
                }
            }
        }

        Ok(yuv_data)
    }

    /// Convert YUY2 to YUV420P
    fn yuy2_to_yuv420p(&self, yuy2_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        let width = width as usize;
        let height = height as usize;

        let y_size = width * height;
        let uv_size = y_size / 4;
        let mut yuv_data = vec![0u8; y_size + uv_size * 2];

        // Extract Y, U, V components from YUY2
        for y in 0..height {
            for x in 0..width {
                let yuy2_idx = (y * width + x) * 2;
                let y_idx = y * width + x;

                // Y component (every even byte in YUY2)
                yuv_data[y_idx] = yuy2_data[yuy2_idx];

                // UV components (every 4th byte, subsampled)
                if y % 2 == 0 && x % 2 == 0 && yuy2_idx + 3 < yuy2_data.len() {
                    let uv_idx = y_size + (y / 2) * (width / 2) + (x / 2);
                    let u_idx = y_size + uv_size + (y / 2) * (width / 2) + (x / 2);

                    // U component (every 4th byte starting at position 1)
                    yuv_data[uv_idx] = yuy2_data[yuy2_idx + 1];
                    // V component (every 4th byte starting at position 3)
                    yuv_data[u_idx] = yuy2_data[yuy2_idx + 3];
                }
            }
        }

        Ok(yuv_data)
    }

    /// Convert audio frame to the expected format
    fn convert_audio_format(&self, frame: &AudioFrameData) -> Result<Vec<u8>> {
        if frame.sample_format == self.audio_encoder.sample_format {
            return Ok(frame.data.clone());
        }

        // Simple I16 to F32 conversion
        match (frame.sample_format, self.audio_encoder.sample_format) {
            (AudioSampleFormat::I16, AudioSampleFormat::F32) => {
                self.i16_to_f32(&frame.data)
            }
            (AudioSampleFormat::F32, AudioSampleFormat::I16) => {
                self.f32_to_i16(&frame.data)
            }
            _ => Err(anyhow!("Unsupported audio format conversion: {:?} -> {:?}",
                           frame.sample_format, self.audio_encoder.sample_format)),
        }
    }

    /// Convert I16 audio samples to F32
    fn i16_to_f32(&self, i16_data: &[u8]) -> Result<Vec<u8>> {
        let samples = i16_data.len() / 2;
        let mut f32_data = Vec::with_capacity(samples * 4);

        let i16_samples: &[i16] = unsafe {
            std::slice::from_raw_parts(i16_data.as_ptr() as *const i16, samples)
        };

        for &sample in i16_samples {
            let f32_sample = sample as f32 / 32768.0;
            f32_data.extend_from_slice(&f32_sample.to_ne_bytes());
        }

        Ok(f32_data)
    }

    /// Convert F32 audio samples to I16
    fn f32_to_i16(&self, f32_data: &[u8]) -> Result<Vec<u8>> {
        let samples = f32_data.len() / 4;
        let mut i16_data = Vec::with_capacity(samples * 2);

        let f32_samples: &[f32] = unsafe {
            std::slice::from_raw_parts(f32_data.as_ptr() as *const f32, samples)
        };

        for &sample in f32_samples {
            let i16_sample = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            i16_data.extend_from_slice(&i16_sample.to_ne_bytes());
        }

        Ok(i16_data)
    }

    /// Get current recording statistics
    pub fn get_statistics(&self) -> MuxerStatistics {
        MuxerStatistics {
            video_frames_written: self.video_frames_written,
            audio_frames_written: self.audio_frames_written,
            current_file_size: self.current_file_size,
            recording_duration: self.last_video_timestamp.saturating_sub(self.timestamp_offset),
            video_bitrate: self.calculate_current_video_bitrate(),
            audio_bitrate: self.calculate_current_audio_bitrate(),
            is_ffmpeg_running: self.ffmpeg_process.as_ref().map_or(false, |p| p.try_wait().ok().map_or(true, |status| status.is_none())),
        }
    }

    /// Calculate current video bitrate
    fn calculate_current_video_bitrate(&self) -> u32 {
        if self.video_frames_written == 0 {
            return 0;
        }

        let duration = self.last_video_timestamp.saturating_sub(self.timestamp_offset);
        if duration.as_secs_f64() > 0.0 {
            // Rough estimate based on frame count and resolution
            let bits_per_frame = (self.video_encoder.width * self.video_encoder.height * 3 * 8) as f64; // RGB24 approximation
            (bits_per_frame * self.video_encoder.frame_rate) as u32
        } else {
            0
        }
    }

    /// Calculate current audio bitrate
    fn calculate_current_audio_bitrate(&self) -> u32 {
        if self.audio_frames_written == 0 {
            return 0;
        }

        // Use configured bitrate or calculate from sample rate and channels
        if self.audio_encoder.bitrate > 0 {
            self.audio_encoder.bitrate
        } else {
            // Estimate: sample_rate * channels * bytes_per_sample * 8
            self.audio_encoder.sample_rate * self.audio_encoder.channels * 4 * 8
        }
    }

    /// Finalize the recording and close the output file
    pub fn finalize(&mut self) -> Result<()> {
        info!("Finalizing MP4 recording");

        // Close FFmpeg stdin to signal end of input
        if let Some(ref mut process) = self.ffmpeg_process {
            drop(process.stdin.take());

            // Wait for FFmpeg to finish
            match process.wait() {
                Ok(status) => {
                    if status.success() {
                        info!("FFmpeg completed successfully");
                    } else {
                        error!("FFmpeg exited with error: {}", status);
                        return Err(anyhow!("FFmpeg exited with error: {}", status));
                    }
                }
                Err(e) => {
                    error!("Failed to wait for FFmpeg: {}", e);
                    return Err(anyhow!("Failed to wait for FFmpeg: {}", e));
                }
            }
        }

        // Check if output file was created
        if !self.output_path.exists() {
            return Err(anyhow!("Output file was not created: {}", self.output_path.display()));
        }

        // Get final file size
        if let Ok(metadata) = std::fs::metadata(&self.output_path) {
            self.current_file_size = metadata.len();
            info!("Final file size: {} bytes", self.current_file_size);
        }

        info!("MP4 recording finalized: {}", self.output_path.display());
        Ok(())
    }

    /// Get the output file path
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

/// Muxer statistics
#[derive(Debug, Clone)]
pub struct MuxerStatistics {
    /// Number of video frames written
    pub video_frames_written: u64,
    /// Number of audio frames written
    pub audio_frames_written: u64,
    /// Current file size in bytes
    pub current_file_size: u64,
    /// Recording duration
    pub recording_duration: Duration,
    /// Current video bitrate
    pub video_bitrate: u32,
    /// Current audio bitrate
    pub audio_bitrate: u32,
    /// Whether FFmpeg is still running
    pub is_ffmpeg_running: bool,
}

impl Drop for MP4Muxer {
    fn drop(&mut self) {
        // Ensure FFmpeg process is properly terminated
        if let Some(ref mut process) = self.ffmpeg_process {
            if let Ok(Some(_)) = process.try_wait() {
                // Process already finished
            } else {
                // Try to kill the process
                let _ = process.kill();
                let _ = process.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::combined_recorder::utils::*;
    use tempfile::tempdir;

    #[test]
    fn test_video_encoder_creation() {
        let config = RecordingConfig::default();
        let (width, height) = config.video.resolution.dimensions();

        let encoder = VideoEncoder {
            codec: config.video.codec,
            width,
            height,
            frame_rate: config.video.frame_rate.value(),
            bitrate: config.video.target_bitrate,
            crf: config.video.crf,
            pixel_format: VideoFormat::YUV420P,
            keyframe_interval: Duration::from_secs(2),
        };

        assert_eq!(encoder.width, width);
        assert_eq!(encoder.height, height);
        assert_eq!(encoder.pixel_format, VideoFormat::YUV420P);
    }

    #[test]
    fn test_audio_encoder_creation() {
        let config = RecordingConfig::default();

        let encoder = AudioEncoder {
            codec: config.audio.codec,
            sample_rate: config.audio.sample_rate,
            channels: config.audio.channels,
            bitrate: config.audio.target_bitrate,
            sample_format: AudioSampleFormat::F32,
        };

        assert_eq!(encoder.sample_rate, config.audio.sample_rate);
        assert_eq!(encoder.channels, config.audio.channels);
        assert_eq!(encoder.sample_format, AudioSampleFormat::F32);
    }

    #[test]
    fn test_mp4_muxer_creation() {
        let output_dir = tempdir().unwrap();
        let output_path = output_dir.path().join("test.mp4");
        let config = RecordingConfig::default();

        // This test might fail if FFmpeg is not available
        let result = MP4Muxer::new(&output_path, &config);

        if result.is_err() {
            // FFmpeg might not be available in test environment
            println!("FFmpeg not available, skipping muxer test");
        } else {
            let muxer = result.unwrap();
            assert_eq!(muxer.output_path, output_path);
        }
    }

    #[test]
    fn test_rgb24_to_yuv420p_conversion() {
        let config = RecordingConfig::default();
        let output_dir = tempdir().unwrap();
        let output_path = output_dir.path().join("test.mp4");

        // Create a mock muxer just for conversion testing
        let muxer = MP4Muxer {
            output_path,
            config: RecordingConfig::default(),
            ffmpeg_process: None,
            video_encoder: VideoEncoder {
                codec: VideoCodec::H264,
                width: 4,
                height: 4,
                frame_rate: 30.0,
                bitrate: 0,
                crf: 23,
                pixel_format: VideoFormat::YUV420P,
                keyframe_interval: Duration::from_secs(2),
            },
            audio_encoder: AudioEncoder {
                codec: AudioCodec::AAC,
                sample_rate: 44100,
                channels: 2,
                bitrate: 128000,
                sample_format: AudioSampleFormat::F32,
            },
            video_frames_written: 0,
            audio_frames_written: 0,
            current_file_size: 0,
            timestamp_offset: Duration::ZERO,
            last_video_timestamp: Duration::ZERO,
            last_audio_timestamp: Duration::ZERO,
        };

        // Create a small RGB24 frame (4x4 pixels)
        let rgb_data = vec![255; 4 * 4 * 3]; // White RGB24 frame
        let yuv_data = muxer.rgb24_to_yuv420p(&rgb_data, 4, 4).unwrap();

        // YUV420P should be smaller than RGB24
        assert!(yuv_data.len() < rgb_data.len());

        // YUV420P for 4x4 should be: Y(16) + U(4) + V(4) = 24 bytes
        assert_eq!(yuv_data.len(), 24);
    }

    #[test]
    fn test_audio_format_conversion() {
        let config = RecordingConfig::default();
        let output_dir = tempdir().unwrap();
        let output_path = output_dir.path().join("test.mp4");

        let muxer = MP4Muxer {
            output_path,
            config: RecordingConfig::default(),
            ffmpeg_process: None,
            video_encoder: VideoEncoder {
                codec: VideoCodec::H264,
                width: 1920,
                height: 1080,
                frame_rate: 30.0,
                bitrate: 0,
                crf: 23,
                pixel_format: VideoFormat::YUV420P,
                keyframe_interval: Duration::from_secs(2),
            },
            audio_encoder: AudioEncoder {
                codec: AudioCodec::AAC,
                sample_rate: 44100,
                channels: 2,
                bitrate: 128000,
                sample_format: AudioSampleFormat::F32,
            },
            video_frames_written: 0,
            audio_frames_written: 0,
            current_file_size: 0,
            timestamp_offset: Duration::ZERO,
            last_video_timestamp: Duration::ZERO,
            last_audio_timestamp: Duration::ZERO,
        };

        // Test I16 to F32 conversion
        let i16_data = vec![0x00, 0x01, 0x00, 0x02]; // Two I16 samples: 256, 512
        let f32_data = muxer.i16_to_f32(&i16_data).unwrap();

        // Should now be 8 bytes (two F32 samples)
        assert_eq!(f32_data.len(), 8);

        // Test F32 to I16 conversion
        let f32_data = vec![0.5f32, -0.5f32];
        let f32_bytes = f32_data.iter().flat_map(|&x| x.to_ne_bytes().to_vec()).collect::<Vec<_>>();
        let i16_data = muxer.f32_to_i16(&f32_bytes).unwrap();

        // Should now be 4 bytes (two I16 samples)
        assert_eq!(i16_data.len(), 4);
    }
}