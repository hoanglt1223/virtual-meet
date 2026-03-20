# Phase 2: Video Streaming via ffmpeg CLI

## Context Links
- `src-tauri/src/virtual_device/webcam.rs` -- VirtualWebcam, DirectShow/MF backends (all stubs)
- `src-tauri/src/virtual_device/webcam.rs:988-997` -- `playback_loop` signature mismatch
- `src-tauri/src/virtual_device/webcam.rs:806-821` -- caller passes single `backend` Arc but fn expects two
- `src-tauri/src/commands.rs:186-227` -- `start_streaming` / `stop_streaming` commands
- `src-tauri/Cargo.toml` -- ffmpeg-next dependency (ffmpeg already required)

## Overview
- **Priority**: P1
- **Status**: pending
- **Description**: Replace impossible userspace virtual webcam driver creation with ffmpeg CLI piping to existing virtual camera (OBS Virtual Camera)

## Key Insight

The DirectShow and MediaFoundation backends try to create virtual webcam **drivers** from userspace code. This is fundamentally impossible without a signed kernel driver. The working approach:

1. Decode video with ffmpeg-next (already works)
2. Pipe raw frames to `ffmpeg` CLI process
3. ffmpeg outputs to OBS Virtual Camera via DirectShow

**ffmpeg command**:
```bash
ffmpeg -re -stream_loop -1 -i "input.mp4" -vcodec rawvideo -pix_fmt yuyv422 -f dshow "video=OBS Virtual Camera"
```

Or simpler -- just let ffmpeg handle the entire decode+output:
```bash
ffmpeg -re -stream_loop -1 -i "input.mp4" -f dshow "video=OBS Virtual Camera"
```

This is the KISS approach. No need for our own frame buffer, decode threads, or playback loop. ffmpeg handles timing, format conversion, everything.

## Architecture

```
[start_streaming command]
        |
        v
[Spawn ffmpeg child process]
  ffmpeg -re -stream_loop -1 -i "video.mp4" -f dshow "video=OBS Virtual Camera"
        |
        v
[OBS Virtual Camera driver] <-- already installed on user machine
        |
        v
[Zoom/Teams/etc sees virtual webcam]
```

State management:
- Store `Child` process handle in state
- `stop_streaming` kills the child process
- No frame buffers, no decode threads, no DirectShow/MF backend code needed

## Related Code Files

### Files to Modify
- `src-tauri/src/commands.rs` -- Rewrite `start_streaming`/`stop_streaming` to manage ffmpeg process
- `src-tauri/src/commands.rs` -- Change `AppState.webcam` to simple process state (or keep VirtualWebcam but gut it)

### Approach: New Simple Webcam State

Rather than gutting the complex VirtualWebcam, create a simpler state struct for the ffmpeg-process approach. Keep the old code but don't use it.

```rust
pub struct VideoStreamState {
    process: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
    current_source: Arc<tokio::sync::Mutex<Option<String>>>,
    ffmpeg_path: String,  // from settings or "ffmpeg" default
    virtual_camera_name: String,  // from settings or "OBS Virtual Camera"
}
```

## Implementation Steps

### Step 1: Create VideoStreamState

In `commands.rs` (or a new `video_stream.rs` if commands.rs gets too big):

```rust
pub struct VideoStreamState {
    process: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
    current_source: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl VideoStreamState {
    pub fn new() -> Self {
        Self {
            process: Arc::new(tokio::sync::Mutex::new(None)),
            current_source: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}
```

### Step 2: Rewrite start_streaming command

```rust
#[tauri::command]
pub async fn start_video_streaming(
    request: VideoStreamRequest,
    state: State<'_, VideoStreamState>,
) -> Result<VideoResponse, String> {
    let mut process = state.process.lock().await;

    // Kill existing process if any
    if let Some(ref mut child) = *process {
        let _ = child.kill().await;
    }

    // Resolve ffmpeg path (check settings.advanced.custom_ffmpeg_path first)
    let ffmpeg_path = "ffmpeg"; // or from settings
    let camera_name = request.camera_name.unwrap_or("OBS Virtual Camera".into());

    // Validate video file exists
    if !std::path::Path::new(&request.path).exists() {
        return Err(format!("Video file not found: {}", request.path));
    }

    // Spawn ffmpeg process
    let child = tokio::process::Command::new(ffmpeg_path)
        .args([
            "-re",                          // Real-time playback speed
            "-stream_loop", "-1",           // Loop forever
            "-i", &request.path,            // Input file
            "-f", "dshow",                  // DirectShow output
            &format!("video={}", camera_name),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())  // Capture errors
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {}. Is ffmpeg in PATH?", e))?;

    *process = Some(child);
    *state.current_source.lock().await = Some(request.path.clone());

    Ok(VideoResponse { success: true, message: "Video streaming started".into(), video_info: None, buffer_status: None })
}
```

### Step 3: Rewrite stop_streaming command

```rust
#[tauri::command]
pub async fn stop_video_streaming(
    state: State<'_, VideoStreamState>,
) -> Result<VideoResponse, String> {
    let mut process = state.process.lock().await;

    if let Some(ref mut child) = *process {
        let _ = child.kill().await;
    }
    *process = None;
    *state.current_source.lock().await = None;

    Ok(VideoResponse { success: true, message: "Video streaming stopped".into(), video_info: None, buffer_status: None })
}
```

### Step 4: Add VideoStreamRequest

```rust
#[derive(Debug, Deserialize)]
pub struct VideoStreamRequest {
    pub path: String,
    pub camera_name: Option<String>,  // e.g. "OBS Virtual Camera"
}
```

### Step 5: Add list_virtual_cameras command

Enumerate DirectShow video devices to find virtual cameras:

```rust
#[tauri::command]
pub async fn list_virtual_cameras() -> Result<Vec<String>, String> {
    // Use ffmpeg to list dshow devices
    let output = tokio::process::Command::new("ffmpeg")
        .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Parse video device names from ffmpeg output
    let devices: Vec<String> = stderr.lines()
        .filter(|l| l.contains("\"") && l.contains("DirectShow video"))
        .filter_map(|l| {
            let start = l.find('"')?;
            let end = l[start+1..].find('"')?;
            Some(l[start+1..start+1+end].to_string())
        })
        .collect();

    Ok(devices)
}
```

### Step 6: Register new state and commands in main.rs

```rust
// In setup:
app.manage(commands::VideoStreamState::new());

// In invoke_handler, replace old start_streaming/stop_streaming:
commands::start_video_streaming,
commands::stop_video_streaming,
commands::list_virtual_cameras,
```

### Step 7: Fix playback_loop signature (cleanup)

The old `playback_loop` on line 989 takes 7 params (both DS and MF backends) but is called on line 813 with a single `backend` Arc. Since we're not using this code path anymore, either:
- Delete `playback_loop` and the old streaming code in `webcam.rs`
- Or fix the signature to match the call site (take a single `backend` + `backend_type`)

**Recommendation**: Keep the old code but mark it `#[allow(dead_code)]` for now. We may want it later if we build a proper virtual camera driver.

### Step 8: Run tests

```bash
cargo test --verbose
cargo clippy --all-targets --all-features -- -D warnings
```

## Todo List

- [ ] Create `VideoStreamState` struct
- [ ] Create `VideoStreamRequest` struct with `camera_name` field
- [ ] Implement `start_video_streaming` command (ffmpeg subprocess)
- [ ] Implement `stop_video_streaming` command (kill subprocess)
- [ ] Implement `list_virtual_cameras` command (ffmpeg device enumeration)
- [ ] Register `VideoStreamState` in `main.rs` setup
- [ ] Register new commands in invoke_handler
- [ ] Fix or suppress `playback_loop` signature mismatch
- [ ] Run `cargo test` and `cargo clippy`

## Success Criteria

- `start_video_streaming` with valid video file + "OBS Virtual Camera" starts ffmpeg subprocess
- Zoom/Teams/Google Meet see the video playing on OBS Virtual Camera
- `stop_video_streaming` kills the subprocess cleanly
- `list_virtual_cameras` returns installed virtual cameras
- ffmpeg missing from PATH returns clear error message
- Existing tests pass

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| ffmpeg dshow output format varies by virtual camera | Test with OBS Virtual Camera; may need `-vcodec rawvideo -pix_fmt yuyv422` flags |
| ffmpeg stderr parsing for device list is fragile | Use regex; fallback to empty list with warning |
| Child process not killed on app crash | Use `Drop` trait on state or Tauri exit handler |
| OBS Virtual Camera requires OBS to be installed | Document; check at startup |

## Security Considerations

- Validate file path before passing to ffmpeg (no shell injection -- using `Command::new` with args, not shell)
- Camera name is user input; sanitize for DirectShow device name format
