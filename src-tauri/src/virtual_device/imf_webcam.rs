//! IMFVirtualCamera backend
//! Decodes video frames via ffmpeg CLI and writes to shared memory for the COM media source.
//! Calls MFCreateVirtualCamera to register the virtual camera with Windows.

use anyhow::{anyhow, Result};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use super::shared_frame_buffer::SharedFrameWriter;
use super::webcam::VideoInfo;

/// CLSID for our custom COM media source (must match the registered DLL).
/// {B4A7E55D-1E7C-4C90-B74A-6D9E3F8A2B10}
const VCAM_SOURCE_CLSID: &str = "{B4A7E55D-1E7C-4C90-B74A-6D9E3F8A2B10}";

/// IMF-based virtual webcam
pub struct ImfWebcam {
    is_active: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    current_source: Arc<Mutex<Option<String>>>,
    video_info: Arc<Mutex<Option<VideoInfo>>>,
    /// Shared between async caller and sync decode thread — use std Mutex
    frame_writer: Arc<StdMutex<Option<SharedFrameWriter>>>,
    ffmpeg_process: Arc<Mutex<Option<Child>>>,
    decode_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl ImfWebcam {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            should_stop: Arc::new(AtomicBool::new(false)),
            current_source: Arc::new(Mutex::new(None)),
            video_info: Arc::new(Mutex::new(None)),
            frame_writer: Arc::new(StdMutex::new(None)),
            ffmpeg_process: Arc::new(Mutex::new(None)),
            decode_thread: Arc::new(Mutex::new(None)),
        }
    }

    /// Check whether IMFVirtualCamera is available on this system.
    /// Requires Windows 11 build 22000+.
    pub fn is_available() -> bool {
        let output = Command::new("cmd")
            .args(["/c", "ver"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok();

        if let Some(output) = output {
            let ver = String::from_utf8_lossy(&output.stdout);
            if let (Some(start), Some(end)) = (ver.find('['), ver.find(']')) {
                let parts: Vec<&str> = ver[start + 1..end].split('.').collect();
                if parts.len() >= 3 {
                    if let Ok(build) = parts[2].trim().parse::<u32>() {
                        return build >= 22000;
                    }
                }
            }
        }
        false
    }

    /// Check whether the COM media source DLL is registered in the Windows registry.
    pub fn is_com_source_registered() -> bool {
        let key = format!(
            "HKLM\\SOFTWARE\\Classes\\CLSID\\{}",
            VCAM_SOURCE_CLSID
        );
        Command::new("reg")
            .args(["query", &key])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Probe video dimensions and frame-rate with ffprobe.
    async fn probe_video(path: &str) -> Result<VideoInfo> {
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

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| anyhow!("Failed to parse ffprobe output: {}", e))?;

        let stream = json["streams"]
            .as_array()
            .and_then(|s| s.first())
            .ok_or_else(|| anyhow!("No video stream found"))?;

        let width = stream["width"].as_u64().unwrap_or(1280) as u32;
        let height = stream["height"].as_u64().unwrap_or(720) as u32;

        let frame_rate = stream["r_frame_rate"]
            .as_str()
            .map(|s| {
                if let Some((n, d)) = s.split_once('/') {
                    let n: f64 = n.parse().unwrap_or(30.0);
                    let d: f64 = d.parse().unwrap_or(1.0);
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

    /// Start decoding video and writing frames to shared memory.
    pub async fn start_streaming(&self, video_path: &str) -> Result<()> {
        if self.is_active.load(Ordering::Relaxed) {
            return Err(anyhow!("IMF webcam already streaming"));
        }

        if !std::path::Path::new(video_path).exists() {
            return Err(anyhow!("Video file not found: {}", video_path));
        }

        let info = Self::probe_video(video_path).await?;
        info!(
            "IMF webcam: {}x{} @ {:.1} fps",
            info.width, info.height, info.frame_rate
        );

        // Create shared memory writer
        let writer = SharedFrameWriter::create(info.width, info.height)?;
        *self.frame_writer.lock().unwrap() = Some(writer);

        *self.video_info.lock().await = Some(info.clone());
        *self.current_source.lock().await = Some(video_path.to_string());

        self.should_stop.store(false, Ordering::Relaxed);
        self.is_active.store(true, Ordering::Relaxed);

        // Spawn the decode thread; it owns the ffmpeg child process
        let video_path_owned = video_path.to_string();
        let width = info.width;
        let height = info.height;
        let frame_rate = info.frame_rate;
        let should_stop = Arc::clone(&self.should_stop);
        let frame_writer = Arc::clone(&self.frame_writer);
        let is_active = Arc::clone(&self.is_active);

        let handle = thread::spawn(move || {
            Self::decode_loop(
                video_path_owned,
                width,
                height,
                frame_rate,
                should_stop,
                frame_writer,
                is_active,
            );
        });

        *self.decode_thread.lock().await = Some(handle);

        info!("IMF webcam streaming started for: {}", video_path);
        Ok(())
    }

    /// Blocking decode loop — runs in a dedicated OS thread.
    fn decode_loop(
        video_path: String,
        width: u32,
        height: u32,
        frame_rate: f64,
        should_stop: Arc<AtomicBool>,
        frame_writer: Arc<StdMutex<Option<SharedFrameWriter>>>,
        is_active: Arc<AtomicBool>,
    ) {
        use std::io::Read;

        let frame_size = (width * height * 4) as usize;
        let mut frame_number: u64 = 0;

        'outer: loop {
            if should_stop.load(Ordering::Relaxed) {
                break;
            }

            // Each iteration spawns a fresh ffmpeg that loops the file
            let mut child = match Command::new("ffmpeg")
                .args([
                    "-re",
                    "-stream_loop", "-1",
                    "-i", &video_path,
                    "-f", "rawvideo",
                    "-pix_fmt", "bgra",
                    "-s", &format!("{}x{}", width, height),
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    error!("IMF: failed to spawn ffmpeg decoder: {}", e);
                    break;
                }
            };

            let mut stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    error!("IMF: no stdout from ffmpeg");
                    let _ = child.kill();
                    break;
                }
            };

            let mut buf = vec![0u8; frame_size];

            loop {
                if should_stop.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break 'outer;
                }

                match stdout.read_exact(&mut buf) {
                    Ok(()) => {
                        let timestamp =
                            (frame_number as f64 / frame_rate * 10_000_000.0) as i64;

                        if let Ok(lock) = frame_writer.lock() {
                            if let Some(ref writer) = *lock {
                                if let Err(e) = writer.write_frame(
                                    width,
                                    height,
                                    &buf,
                                    frame_number,
                                    timestamp,
                                ) {
                                    error!("IMF: write_frame failed: {}", e);
                                }
                            }
                        }

                        frame_number += 1;
                    }
                    Err(e) => {
                        if should_stop.load(Ordering::Relaxed) {
                            let _ = child.kill();
                            let _ = child.wait();
                            break 'outer;
                        }
                        warn!("IMF: ffmpeg read error (will restart): {}", e);
                        break; // restart ffmpeg in outer loop
                    }
                }
            }

            let _ = child.kill();
            let _ = child.wait();

            if should_stop.load(Ordering::Relaxed) {
                break;
            }

            // Short pause before restarting
            thread::sleep(Duration::from_millis(100));
        }

        is_active.store(false, Ordering::Relaxed);
        info!("IMF decode loop exited after {} frames", frame_number);
    }

    pub async fn stop_streaming(&self) -> Result<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping IMF webcam");
        self.should_stop.store(true, Ordering::Relaxed);

        // Kill any separately tracked ffmpeg process (belt-and-suspenders)
        if let Some(mut child) = self.ffmpeg_process.lock().await.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        // Join the decode thread
        if let Some(handle) = self.decode_thread.lock().await.take() {
            let _ = handle.join();
        }

        // Release shared memory
        *self.frame_writer.lock().unwrap() = None;
        *self.current_source.lock().await = None;
        *self.video_info.lock().await = None;
        self.is_active.store(false, Ordering::Relaxed);

        info!("IMF webcam stopped");
        Ok(())
    }

    pub fn is_active_sync(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    pub async fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    pub async fn current_source(&self) -> Option<String> {
        self.current_source.lock().await.clone()
    }

    pub async fn get_video_info(&self) -> Result<Option<VideoInfo>> {
        Ok(self.video_info.lock().await.clone())
    }
}

impl Default for ImfWebcam {
    fn default() -> Self {
        Self::new()
    }
}
