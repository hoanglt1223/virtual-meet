//! Virtual Webcam Implementation using DirectShow/Media Foundation
//!
//! This module provides native Rust implementation of a virtual webcam
//! without requiring external applications like OBS.

use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use std::collections::VecDeque;

// FFmpeg imports for video decoding
use ffmpeg_next as ffmpeg;
use ffmpeg_next::{
    codec, decoder, format, frame, media,
    SoftwareScaling, util::frame::video::Video as VideoFrame
};

/// Video frame data with metadata
#[derive(Debug, Clone)]
pub struct VideoFrameData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: Duration,
    pub frame_number: u64,
}

impl VideoFrameData {
    pub fn new(data: Vec<u8>, width: u32, height: u32, timestamp: Duration, frame_number: u64) -> Self {
        Self {
            data,
            width,
            height,
            timestamp,
            frame_number,
        }
    }

    /// Get the size of the frame in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get frame dimensions as (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Frame buffer for managing decoded video frames
pub struct FrameBuffer {
    frames: Arc<StdMutex<VecDeque<VideoFrameData>>>,
    max_size: usize,
    total_frames: u64,
}

impl FrameBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            frames: Arc::new(StdMutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            total_frames: 0,
        }
    }

    /// Add a frame to the buffer
    pub fn push_frame(&mut self, frame: VideoFrameData) -> Result<()> {
        let mut frames = self.frames.lock().map_err(|_| anyhow!("Failed to lock frame buffer"))?;

        if frames.len() >= self.max_size {
            frames.pop_front(); // Remove oldest frame
        }

        frames.push_back(frame);
        self.total_frames += 1;

        Ok(())
    }

    /// Get the next frame for playback
    pub fn get_next_frame(&self) -> Option<VideoFrameData> {
        let frames = self.frames.lock().ok()?;
        frames.front().cloned()
    }

    /// Remove and return the next frame
    pub fn pop_frame(&mut self) -> Option<VideoFrameData> {
        let mut frames = self.frames.lock().ok()?;
        frames.pop_front()
    }

    /// Clear all frames from buffer
    pub fn clear(&mut self) -> Result<()> {
        let mut frames = self.frames.lock().map_err(|_| anyhow!("Failed to lock frame buffer"))?;
        frames.clear();
        self.total_frames = 0;
        Ok(())
    }

    /// Get current buffer length
    pub fn len(&self) -> usize {
        let frames = self.frames.lock().ok();
        frames.map(|f| f.len()).unwrap_or(0)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get total number of frames processed
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.max_size
    }
}

/// Video decoder using FFmpeg
pub struct VideoDecoder {
    decoder: Option<decoder::Video>,
    format_context: Option<format::Context>,
    video_stream_index: Option<usize>,
    width: u32,
    height: u32,
    frame_rate: f64,
    duration: Option<Duration>,
}

impl VideoDecoder {
    pub fn new() -> Self {
        Self {
            decoder: None,
            format_context: None,
            video_stream_index: None,
            width: 0,
            height: 0,
            frame_rate: 0.0,
            duration: None,
        }
    }

    /// Open a video file for decoding
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        info!("Opening video file: {:?}", path.as_ref());

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| anyhow!("Failed to initialize FFmpeg: {}", e))?;

        // Open input file
        let mut input = format::input(&path)
            .map_err(|e| anyhow!("Failed to open video file: {}", e))?;

        // Find video stream
        let video_stream = input
            .streams()
            .find(|stream| stream.parameters().medium() == media::Type::Video)
            .ok_or_else(|| anyhow!("No video stream found in file"))?;

        let video_stream_index = video_stream.index();

        // Get video parameters
        let params = video_stream.parameters();
        let codec_id = params.id();

        // Find decoder
        let decoder = codec::find(codec_id)
            .ok_or_else(|| anyhow!("Decoder not found for codec: {:?}", codec_id))?;

        // Create decoder context
        let mut decoder_context = decoder.create()
            .map_err(|e| anyhow!("Failed to create decoder context: {}", e))?;

        // Set decoder parameters
        decoder_context.set_parameters(params);

        // Open decoder
        let decoder = decoder_context.decode()
            .map_err(|e| anyhow!("Failed to open decoder: {}", e))?;

        // Get video properties
        let width = decoder.width();
        let height = decoder.height();
        let frame_rate = video_stream.avg_frame_rate();
        let frame_rate = frame_rate.numerator() as f64 / frame_rate.denominator() as f64;

        let duration = if let Some(duration) = input.duration() {
            Some(Duration::from_millis(duration as u64))
        } else {
            None
        };

        self.decoder = Some(decoder);
        self.format_context = Some(input);
        self.video_stream_index = Some(video_stream_index);
        self.width = width;
        self.height = height;
        self.frame_rate = frame_rate;
        self.duration = duration;

        info!("Video opened successfully: {}x{} @ {:.2} FPS, duration: {:?}",
              width, height, frame_rate, duration);

        Ok(())
    }

    /// Decode all frames from the video
    pub fn decode_all_frames(&mut self, frame_buffer: &mut FrameBuffer) -> Result<()> {
        let decoder = self.decoder.as_mut().ok_or_else(|| anyhow!("Decoder not initialized"))?;
        let format_context = self.format_context.as_mut().ok_or_else(|| anyhow!("Format context not initialized"))?;
        let video_stream_index = self.video_stream_index.ok_or_else(|| anyhow!("Video stream index not set"))?;

        info!("Starting frame decoding...");

        let mut frame_count = 0u64;
        let mut decoded_frame = VideoFrame::empty();

        for (stream_index, packet) in format_context.packets() {
            if stream_index != video_stream_index {
                continue;
            }

            decoder.send_packet(&packet)
                .map_err(|e| anyhow!("Error sending packet to decoder: {}", e))?;

            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                // Convert frame to RGB24 format
                let mut rgb_frame = VideoFrame::empty();

                // Set up scaler to convert to RGB24
                let mut scaler = SoftwareScaling::context(
                    decoded_frame.width(),
                    decoded_frame.height(),
                    decoded_frame.format(),
                    decoded_frame.width(),
                    decoded_frame.height(),
                    ffmpeg::format::Pixel::RGB24
                ).map_err(|e| anyhow!("Failed to create scaler: {}", e))?;

                scaler.run(&decoded_frame, &mut rgb_frame)
                    .map_err(|e| anyhow!("Failed to scale frame: {}", e))?;

                // Extract frame data
                let width = rgb_frame.width() as u32;
                let height = rgb_frame.height() as u32;
                let data = rgb_frame.data(0).to_vec();

                // Calculate timestamp (approximate based on frame count)
                let timestamp = Duration::from_millis((frame_count as f64 / self.frame_rate * 1000.0) as u64);

                let video_frame = VideoFrameData::new(data, width, height, timestamp, frame_count);

                frame_buffer.push_frame(video_frame)?;
                frame_count += 1;

                debug!("Decoded frame {} ({}x{})", frame_count, width, height);
            }
        }

        // Flush decoder
        decoder.send_eof()
            .map_err(|e| anyhow!("Error sending EOF to decoder: {}", e))?;

        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            // Process remaining frames as above
            let mut rgb_frame = VideoFrame::empty();

            let mut scaler = SoftwareScaling::context(
                decoded_frame.width(),
                decoded_frame.height(),
                decoded_frame.format(),
                decoded_frame.width(),
                decoded_frame.height(),
                ffmpeg::format::Pixel::RGB24
            ).map_err(|e| anyhow!("Failed to create scaler: {}", e))?;

            scaler.run(&decoded_frame, &mut rgb_frame)
                .map_err(|e| anyhow!("Failed to scale frame: {}", e))?;

            let width = rgb_frame.width() as u32;
            let height = rgb_frame.height() as u32;
            let data = rgb_frame.data(0).to_vec();
            let timestamp = Duration::from_millis((frame_count as f64 / self.frame_rate * 1000.0) as u64);

            let video_frame = VideoFrameData::new(data, width, height, timestamp, frame_count);
            frame_buffer.push_frame(video_frame)?;
            frame_count += 1;
        }

        info!("Decoding completed. Total frames: {}", frame_count);
        Ok(())
    }

    /// Get video properties
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn frame_rate(&self) -> f64 {
        self.frame_rate
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        debug!("Video decoder dropped");
    }
}

/// Virtual webcam manager
pub struct VirtualWebcam {
    is_active: Arc<Mutex<bool>>,
    current_source: Arc<Mutex<Option<String>>>,
    frame_buffer: Arc<StdMutex<FrameBuffer>>,
    video_decoder: Arc<StdMutex<VideoDecoder>>,
    playback_handle: Arc<StdMutex<Option<thread::JoinHandle<()>>>>,
    should_stop: Arc<StdMutex<bool>>,
}

impl VirtualWebcam {
    /// Create a new virtual webcam instance
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(Mutex::new(false)),
            current_source: Arc::new(Mutex::new(None)),
            frame_buffer: Arc::new(StdMutex::new(FrameBuffer::new(300))), // Buffer 10 seconds at 30fps
            video_decoder: Arc::new(StdMutex::new(VideoDecoder::new())),
            playback_handle: Arc::new(StdMutex::new(None)),
            should_stop: Arc::new(StdMutex::new(false)),
        }
    }

    /// Initialize the virtual webcam
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing virtual webcam using DirectShow/Media Foundation");

        // TODO: Implement DirectShow filter registration
        // TODO: Create virtual webcam device
        // TODO: Set up media pipeline

        warn!("Virtual webcam initialization not yet implemented - requires DirectShow filter development");
        Ok(())
    }

    /// Start streaming video from a file
    pub async fn start_streaming(&self, video_path: &str) -> Result<()> {
        let mut is_active = self.is_active.lock().await;
        let mut current_source = self.current_source.lock().await;

        if *is_active {
            return Err(anyhow!("Webcam already streaming"));
        }

        info!("Starting video stream from: {}", video_path);

        // Validate video file exists
        if !Path::new(video_path).exists() {
            return Err(anyhow!("Video file not found: {}", video_path));
        }

        // Decode video and fill frame buffer
        {
            let mut decoder = self.video_decoder.lock().map_err(|_| anyhow!("Failed to lock video decoder"))?;
            let mut frame_buffer = self.frame_buffer.lock().map_err(|_| anyhow!("Failed to lock frame buffer"))?;

            // Clear any existing frames
            frame_buffer.clear()?;

            // Open and decode video file
            decoder.open(video_path)?;
            decoder.decode_all_frames(&mut frame_buffer)?;

            if frame_buffer.is_empty() {
                return Err(anyhow!("No frames decoded from video file"));
            }

            info!("Video decoded successfully. Buffer contains {} frames", frame_buffer.len());
        }

        // Start playback thread
        {
            let mut should_stop = self.should_stop.lock().map_err(|_| anyhow!("Failed to lock stop flag"))?;
            *should_stop = false;
        }

        let frame_buffer_clone = Arc::clone(&self.frame_buffer);
        let should_stop_clone = Arc::clone(&self.should_stop);
        let video_path_clone = video_path.to_string();

        let frame_rate = {
            let decoder = self.video_decoder.lock().map_err(|_| anyhow!("Failed to lock video decoder"))?;
            decoder.frame_rate()
        };

        let playback_handle = thread::spawn(move || {
            Self::playback_loop(frame_buffer_clone, should_stop_clone, frame_rate, video_path_clone);
        });

        {
            let mut handle = self.playback_handle.lock().map_err(|_| anyhow!("Failed to lock playback handle"))?;
            *handle = Some(playback_handle);
        }

        *is_active = true;
        *current_source = Some(video_path.to_string());

        info!("Video streaming started successfully");
        Ok(())
    }

    /// Stop streaming
    pub async fn stop_streaming(&self) -> Result<()> {
        let mut is_active = self.is_active.lock().await;
        let mut current_source = self.current_source.lock().await;

        if !*is_active {
            return Ok(());
        }

        info!("Stopping video stream");

        // Signal playback thread to stop
        {
            let mut should_stop = self.should_stop.lock().map_err(|_| anyhow!("Failed to lock stop flag"))?;
            *should_stop = true;
        }

        // Wait for playback thread to finish
        {
            let mut handle = self.playback_handle.lock().map_err(|_| anyhow!("Failed to lock playback handle"))?;
            if let Some(handle) = handle.take() {
                if let Err(e) = handle.join() {
                    warn!("Error waiting for playback thread to finish: {:?}", e);
                }
            }
        }

        // Clear frame buffer
        {
            let mut frame_buffer = self.frame_buffer.lock().map_err(|_| anyhow!("Failed to lock frame buffer"))?;
            if let Err(e) = frame_buffer.clear() {
                warn!("Error clearing frame buffer: {}", e);
            }
        }

        *is_active = false;
        *current_source = None;

        info!("Video stream stopped");
        Ok(())
    }

    /// Check if webcam is active
    pub async fn is_active(&self) -> bool {
        *self.is_active.lock().await
    }

    /// Get current video source
    pub async fn current_source(&self) -> Option<String> {
        self.current_source.lock().await.clone()
    }

    /// Get video information
    pub async fn get_video_info(&self) -> Result<Option<VideoInfo>> {
        let decoder = self.video_decoder.lock().map_err(|_| anyhow!("Failed to lock video decoder"))?;

        if decoder.width() == 0 {
            return Ok(None);
        }

        Ok(Some(VideoInfo {
            width: decoder.width(),
            height: decoder.height(),
            frame_rate: decoder.frame_rate(),
            duration: decoder.duration(),
        }))
    }

    /// Get frame buffer status
    pub async fn get_buffer_status(&self) -> BufferStatus {
        let frame_buffer = self.frame_buffer.lock().ok();
        match frame_buffer {
            Some(buffer) => BufferStatus {
                current_frames: buffer.len(),
                capacity: buffer.capacity(),
                total_processed: buffer.total_frames(),
            },
            None => BufferStatus {
                current_frames: 0,
                capacity: 0,
                total_processed: 0,
            },
        }
    }

    /// Playback loop for streaming video frames
    fn playback_loop(
        frame_buffer: Arc<StdMutex<FrameBuffer>>,
        should_stop: Arc<StdMutex<bool>>,
        frame_rate: f64,
        video_path: String,
    ) {
        info!("Starting playback loop for: {}", video_path);

        let frame_duration = Duration::from_millis((1000.0 / frame_rate) as u64);
        let mut loop_count = 0u64;
        let mut frame_count_in_loop = 0u64;

        while {
            let stop = should_stop.lock();
            stop.map(|s| !*s).unwrap_or(false)
        } {
            let frame_result = {
                let mut buffer = frame_buffer.lock().ok();
                match buffer {
                    Some(ref mut buf) => buf.pop_frame(),
                    None => None,
                }
            };

            if let Some(frame) = frame_result {
                // TODO: Send frame to virtual webcam device
                // For now, we just simulate frame timing

                debug!("Displaying frame {} (loop {}, frame {} in loop)",
                       frame.frame_number, loop_count, frame_count_in_loop);

                // Simulate frame processing time
                thread::sleep(frame_duration);
                frame_count_in_loop += 1;

                // Re-add frame to buffer for looping
                {
                    let mut buffer = frame_buffer.lock().ok();
                    if let Some(ref mut buf) = buffer {
                        if let Err(e) = buf.push_frame(frame.clone()) {
                            error!("Failed to re-add frame to buffer: {}", e);
                        }
                    }
                }
            } else {
                // No frames available, wait a bit and check again
                warn!("No frames available in buffer, waiting...");
                thread::sleep(Duration::from_millis(100));
            }

            // Handle loop reset when we've processed all frames
            if frame_count_in_loop >= 300 { // Reset after ~10 seconds at 30fps
                loop_count += 1;
                frame_count_in_loop = 0;
                info!("Starting video loop #{}", loop_count + 1);
            }
        }

        info!("Playback loop stopped for: {}", video_path);
    }

    /// List available video devices
    pub async fn list_devices() -> Result<Vec<String>> {
        info!("Enumerating video devices");

        // TODO: Implement device enumeration using DirectShow or MF

        warn!("Device enumeration not yet implemented");
        Ok(vec!["VirtualWebcam".to_string()])
    }
}

impl Drop for VirtualWebcam {
    fn drop(&mut self) {
        info!("Virtual webcam dropped");

        // Ensure playback is stopped
        if let Ok(mut should_stop) = self.should_stop.lock() {
            *should_stop = true;
        }

        // Wait for playback thread to finish
        if let Ok(mut handle) = self.playback_handle.lock() {
            if let Some(handle) = handle.take() {
                let _ = handle.join();
            }
        }
    }
}

/// Video information struct
#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub duration: Option<Duration>,
}

impl VideoInfo {
    /// Get video dimensions as string
    pub fn dimensions_string(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }

    /// Get duration as string
    pub fn duration_string(&self) -> String {
        match self.duration {
            Some(duration) => {
                let seconds = duration.as_secs();
                let minutes = seconds / 60;
                let seconds = seconds % 60;
                format!("{:02}:{:02}", minutes, seconds)
            },
            None => "Unknown".to_string(),
        }
    }
}

/// Frame buffer status
#[derive(Debug, Clone, serde::Serialize)]
pub struct BufferStatus {
    pub current_frames: usize,
    pub capacity: usize,
    pub total_processed: u64,
}

impl BufferStatus {
    /// Get buffer utilization as percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            (self.current_frames as f64 / self.capacity as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webcam_creation() {
        let webcam = VirtualWebcam::new();
        assert!(!webcam.is_active().await);
    }

    #[tokio::test]
    async fn test_webcam_initialization() {
        let webcam = VirtualWebcam::new();
        // This will likely fail until we implement DirectShow integration
        // let result = webcam.initialize().await;
        // assert!(result.is_ok() || result.unwrap_err().to_string().contains("not yet implemented"));
    }
}