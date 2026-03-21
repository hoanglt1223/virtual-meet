//! Virtual Webcam - streams video to virtual camera via ffmpeg CLI (OBS mode)
//! or via IMFVirtualCamera shared-memory pipeline (IMF mode).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

use super::imf_webcam::ImfWebcam;

/// Video information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    #[serde(skip)]
    pub duration: Option<Duration>,
}

/// Frame buffer status (kept for API compat)
#[derive(Debug, Clone, Serialize)]
pub struct BufferStatus {
    pub current_frames: usize,
    pub capacity: usize,
    pub total_processed: u64,
}

/// Webcam backend selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebcamMode {
    /// Stream via ffmpeg CLI to OBS Virtual Camera (DirectShow)
    Obs,
    /// Stream via IMFVirtualCamera (Windows 11 built-in, no OBS needed)
    Imf,
}

impl Default for WebcamMode {
    fn default() -> Self {
        // Prefer IMF when the OS supports it; fall back to OBS
        if ImfWebcam::is_available() {
            WebcamMode::Imf
        } else {
            WebcamMode::Obs
        }
    }
}

/// Virtual webcam that supports two backends: OBS (ffmpeg→DirectShow) and IMF
pub struct VirtualWebcam {
    // --- OBS backend state ---
    is_active: Arc<AtomicBool>,
    current_source: Arc<Mutex<Option<String>>>,
    ffmpeg_process: Arc<Mutex<Option<Child>>>,
    video_info: Arc<Mutex<Option<VideoInfo>>>,
    target_device: Arc<Mutex<Option<String>>>,
    // --- mode + IMF backend ---
    mode: WebcamMode,
    imf_webcam: Arc<Mutex<ImfWebcam>>,
}

impl VirtualWebcam {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            current_source: Arc::new(Mutex::new(None)),
            ffmpeg_process: Arc::new(Mutex::new(None)),
            video_info: Arc::new(Mutex::new(None)),
            target_device: Arc::new(Mutex::new(None)),
            mode: WebcamMode::default(),
            imf_webcam: Arc::new(Mutex::new(ImfWebcam::new())),
        }
    }

    /// Check if ffmpeg is available (required for both backends)
    pub async fn initialize(&self) -> Result<()> {
        info!("Checking ffmpeg availability for virtual webcam");
        let output = Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("ffmpeg not found. Install ffmpeg and ensure it's in PATH: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!("ffmpeg check failed"));
        }

        info!("ffmpeg is available for virtual webcam (mode: {:?})", self.mode);
        Ok(())
    }

    /// Switch the active backend mode. Has no effect while streaming.
    pub async fn set_mode(&mut self, mode: WebcamMode) {
        self.mode = mode;
        info!("Webcam backend mode set to: {:?}", self.mode);
    }

    /// Returns the current backend mode.
    pub fn get_mode(&self) -> &WebcamMode {
        &self.mode
    }

    // ------------------------------------------------------------------
    // Public streaming API — dispatches to the selected backend
    // ------------------------------------------------------------------

    /// Start streaming — tries IMF first, falls back to OBS if IMF fails.
    pub async fn start_streaming(&self, video_path: &str) -> Result<()> {
        match self.mode {
            WebcamMode::Obs => self.start_streaming_obs(video_path).await,
            WebcamMode::Imf => {
                let imf = self.imf_webcam.lock().await;
                match imf.start_streaming(video_path).await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        info!("IMF mode failed ({}), falling back to OBS mode", e);
                        drop(imf); // release lock
                        self.start_streaming_obs(video_path).await
                    }
                }
            }
        }
    }

    /// Stop streaming — delegates to the active backend.
    pub async fn stop_streaming(&self) -> Result<()> {
        match self.mode {
            WebcamMode::Obs => self.stop_streaming_obs().await,
            WebcamMode::Imf => {
                let imf = self.imf_webcam.lock().await;
                imf.stop_streaming().await
            }
        }
    }

    pub async fn is_active(&self) -> bool {
        match self.mode {
            WebcamMode::Obs => self.is_active.load(Ordering::Relaxed),
            WebcamMode::Imf => self.imf_webcam.lock().await.is_active().await,
        }
    }

    pub async fn current_source(&self) -> Option<String> {
        match self.mode {
            WebcamMode::Obs => self.current_source.lock().await.clone(),
            WebcamMode::Imf => self.imf_webcam.lock().await.current_source().await,
        }
    }

    pub async fn get_video_info(&self) -> Result<Option<VideoInfo>> {
        match self.mode {
            WebcamMode::Obs => Ok(self.video_info.lock().await.clone()),
            WebcamMode::Imf => self.imf_webcam.lock().await.get_video_info().await,
        }
    }

    pub async fn get_buffer_status(&self) -> BufferStatus {
        BufferStatus {
            current_frames: 0,
            capacity: 0,
            total_processed: 0,
        }
    }

    // ------------------------------------------------------------------
    // OBS backend (private)
    // ------------------------------------------------------------------

    /// Probe video file info using ffprobe
    async fn probe_video(&self, path: &str) -> Result<VideoInfo> {
        let output = Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_streams",
                "-select_streams", "v:0",
                path,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("ffprobe failed: {}", e))?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse ffprobe output: {}", e))?;

        let stream = json["streams"]
            .as_array()
            .and_then(|s| s.first())
            .ok_or_else(|| anyhow!("No video stream found"))?;

        let width = stream["width"].as_u64().unwrap_or(0) as u32;
        let height = stream["height"].as_u64().unwrap_or(0) as u32;

        let frame_rate = stream["r_frame_rate"]
            .as_str()
            .map(|s| {
                if let Some((num, den)) = s.split_once('/') {
                    let n: f64 = num.parse().unwrap_or(30.0);
                    let d: f64 = den.parse().unwrap_or(1.0);
                    if d > 0.0 { n / d } else { 30.0 }
                } else {
                    s.parse().unwrap_or(30.0)
                }
            })
            .unwrap_or(30.0);

        let duration = stream["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .map(Duration::from_secs_f64);

        Ok(VideoInfo { width, height, frame_rate, duration })
    }

    /// OBS backend: stream video to a DirectShow virtual camera via ffmpeg
    async fn start_streaming_obs(&self, video_path: &str) -> Result<()> {
        if self.is_active.load(Ordering::Relaxed) {
            return Err(anyhow!("Already streaming (OBS mode)"));
        }

        if !std::path::Path::new(video_path).exists() {
            return Err(anyhow!("Video file not found: {}", video_path));
        }

        let info = self.probe_video(video_path).await?;
        info!("OBS mode: {}x{} @ {:.1} fps", info.width, info.height, info.frame_rate);
        *self.video_info.lock().await = Some(info);

        let target = self
            .target_device
            .lock()
            .await
            .clone()
            .unwrap_or_else(|| "OBS Virtual Camera".to_string());

        info!("Streaming {} → virtual camera: {}", video_path, target);

        let child = Command::new("ffmpeg")
            .args([
                "-re",
                "-stream_loop", "-1",
                "-i", video_path,
                "-f", "dshow",
                "-vcodec", "rawvideo",
                "-pix_fmt", "yuyv422",
                &format!("video={}", target),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start ffmpeg: {}", e))?;

        *self.ffmpeg_process.lock().await = Some(child);
        *self.current_source.lock().await = Some(video_path.to_string());
        self.is_active.store(true, Ordering::Relaxed);

        info!("OBS video streaming started");
        Ok(())
    }

    /// OBS backend: stop ffmpeg streaming
    async fn stop_streaming_obs(&self) -> Result<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping OBS video stream");

        if let Some(mut child) = self.ffmpeg_process.lock().await.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        *self.current_source.lock().await = None;
        *self.video_info.lock().await = None;
        self.is_active.store(false, Ordering::Relaxed);

        info!("OBS video stream stopped");
        Ok(())
    }

    // ------------------------------------------------------------------
    // Shared utilities
    // ------------------------------------------------------------------

    /// Set target virtual camera device (OBS backend)
    pub async fn set_target_device(&self, device: String) {
        *self.target_device.lock().await = Some(device);
    }

    /// List available video devices using ffmpeg DirectShow enumeration
    pub async fn list_devices() -> Result<Vec<String>> {
        info!("Listing video devices via ffmpeg");
        let output = Command::new("ffmpeg")
            .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("Failed to list devices: {}", e))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut devices = Vec::new();
        let mut in_video_section = false;

        for line in stderr.lines() {
            if line.contains("DirectShow video devices") {
                in_video_section = true;
                continue;
            }
            if line.contains("DirectShow audio devices") {
                in_video_section = false;
                continue;
            }
            if in_video_section {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line.rfind('"') {
                        if end > start {
                            let name = &line[start + 1..end];
                            if !name.contains("Alternative name") {
                                devices.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        info!("Found {} video devices", devices.len());
        Ok(devices)
    }
}

impl Default for VirtualWebcam {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VirtualWebcam {
    fn drop(&mut self) {
        // Kill OBS ffmpeg process on drop
        if let Ok(mut proc) = self.ffmpeg_process.try_lock() {
            if let Some(mut child) = proc.take() {
                let _ = child.kill();
            }
        }
        // IMF webcam cleans up via its own Drop
    }
}
