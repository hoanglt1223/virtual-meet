# Phase 1: Audio Playback to Selected Device

## Context Links
- `src-tauri/src/commands.rs:350-379` -- stubbed `start_audio_streaming` / `stop_audio_streaming`
- `src-tauri/src/commands.rs:27-40` -- `AppState` with `Arc<VirtualMicrophone>` (immutable)
- `src-tauri/src/virtual_device/microphone.rs` -- full VirtualMicrophone implementation
- `src-tauri/src/audio.rs` -- AudioFrameData, AudioConfig, symphonia decoding
- `src-tauri/src/audio_decoder.rs` -- AudioDecoder with `decode_all`

## Overview
- **Priority**: P1 (easiest win, highest user value)
- **Status**: pending
- **Description**: Replace the stubbed audio streaming commands with working playback to a user-selected output device via rodio/cpal

## Key Insight

The existing `VirtualMicrophone` tries to create a kernel-level virtual audio device (WASAPI loopback, KS filter) -- **impossible from userspace**. But the `start_playback_thread()` method already has working cpal output stream code that plays to the **default** output device. The fix is:

1. Let user select an output device (e.g., "CABLE Input (VB-Audio)")
2. Play decoded audio to that specific device via cpal
3. Meeting apps pick up VB-CABLE Output as a microphone

This is just **audio playback to a non-default device**. No virtual driver needed.

## Architecture

```
[Audio File] -> [symphonia decode] -> [AudioBuffer] -> [cpal output stream] -> [Selected Device]
                                                                                    |
                                                                          e.g. "CABLE Input"
                                                                                    |
                                                                          Meeting app sees as mic
```

## Related Code Files

### Files to Modify
- `src-tauri/src/commands.rs` -- Fix `start_audio_streaming`, `stop_audio_streaming`, add `AudioStreamRequest` with device_name field
- `src-tauri/src/virtual_device/microphone.rs` -- Add `start_streaming_to_device(&mut self, path, device_name)` method; refactor `start_playback_thread` to accept a target device
- `src-tauri/src/main.rs` -- Add new commands to handler list if needed (list_output_devices)

### Files to Create
- None (modify existing only)

## Implementation Steps

### Step 1: Add device selection to audio playback

In `microphone.rs`, modify `start_playback_thread` to accept an optional device name:

```rust
// In VirtualMicrophone
pub async fn start_streaming_to_device(&mut self, audio_path: &str, device_name: Option<&str>) -> Result<()> {
    // ... existing validation ...

    // Find the target device by name
    let host = cpal::default_host();
    let device = if let Some(name) = device_name {
        host.output_devices()?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| anyhow!("Output device '{}' not found", name))?
    } else {
        host.default_output_device()
            .ok_or_else(|| anyhow!("No default output device"))?
    };

    // Build output stream on the selected device (same as existing code)
    // ... rest of start_playback_thread logic using `device` ...
}
```

### Step 2: Fix AppState mutability

Change `AppState` to use interior mutability for the microphone:

```rust
// In commands.rs
pub struct AppState {
    pub webcam: Arc<VirtualWebcam>,
    pub microphone: Arc<tokio::sync::Mutex<VirtualMicrophone>>,  // Changed from Arc<VirtualMicrophone>
    pub device_enumerator: Arc<DeviceEnumerator>,
}
```

This allows `start_audio_streaming` to get `&mut` access:

```rust
#[tauri::command]
pub async fn start_audio_streaming(
    request: AudioStreamRequest,
    state: State<'_, AppState>,
) -> Result<VideoResponse, String> {
    let mut mic = state.microphone.lock().await;
    mic.start_streaming_to_device(&request.path, request.device_name.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    // ... return success response ...
}
```

### Step 3: Add AudioStreamRequest with device field

```rust
#[derive(Debug, Deserialize)]
pub struct AudioStreamRequest {
    pub path: String,
    pub device_name: Option<String>,  // e.g. "CABLE Input (VB-Audio Virtual Cable)"
}
```

### Step 4: Strip out WASAPI/KS virtual driver code from the hot path

In `start_playback_thread`, remove the `tokio::runtime::Runtime::new()` call inside the cpal callback (line 660) that sends samples to the WASAPI backend. The cpal stream callback should ONLY fill the output buffer from the decoded audio buffer. The WASAPI/KS backends are dead code for now.

The existing `create_virtual_audio_stream` at line 580 already does the right thing for playback -- it reads from `AudioBuffer`, processes, and fills the output. Just remove the backend forwarding (lines 660-697) and it works.

### Step 5: Add list_output_devices command

```rust
#[tauri::command]
pub async fn list_output_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let devices: Vec<String> = host.output_devices()
        .map_err(|e| e.to_string())?
        .filter_map(|d| d.name().ok())
        .collect();
    Ok(devices)
}
```

Register in `main.rs` invoke_handler.

### Step 6: Fix stop_audio_streaming

```rust
#[tauri::command]
pub async fn stop_audio_streaming(state: State<'_, AppState>) -> Result<VideoResponse, String> {
    let mut mic = state.microphone.lock().await;
    mic.stop_streaming().await.map_err(|e| e.to_string())?;
    Ok(VideoResponse { success: true, message: "Stopped".into(), video_info: None, buffer_status: None })
}
```

### Step 7: Fix all callers of `state.microphone`

After changing from `Arc<VirtualMicrophone>` to `Arc<Mutex<VirtualMicrophone>>`, update:
- `commands.rs` -- `init_microphone`, `get_microphone_status`, `set_microphone_volume`, `set_microphone_muted`, `toggle_microphone_mute`
- `main.rs:101-108` -- initialization spawn block

Each changes from `state.microphone.method()` to `state.microphone.lock().await.method()`.

### Step 8: Run tests

```bash
cargo test --verbose
cargo clippy --all-targets --all-features -- -D warnings
```

## Todo List

- [ ] Add `AudioStreamRequest` struct with `device_name` field
- [ ] Change `AppState.microphone` to `Arc<tokio::sync::Mutex<VirtualMicrophone>>`
- [ ] Add `start_streaming_to_device` method to `VirtualMicrophone`
- [ ] Implement `start_audio_streaming` command (remove TODO stub)
- [ ] Implement `stop_audio_streaming` command (remove TODO stub)
- [ ] Remove WASAPI/KS backend calls from cpal stream callback
- [ ] Add `list_output_devices` command
- [ ] Fix all callers of `state.microphone` for new Mutex wrapper
- [ ] Fix `main.rs` microphone initialization
- [ ] Run `cargo test` and `cargo clippy`

## Success Criteria

- `start_audio_streaming` with a valid audio file and device name plays audio through that device
- `stop_audio_streaming` stops playback cleanly
- `list_output_devices` returns actual system output devices including VB-CABLE
- All existing tests pass
- No clippy warnings

## Security Considerations

- Validate file paths before passing to decoder (prevent path traversal)
- Device name is user-provided string; validate it exists before using

## Next Steps

- Phase 2 uses similar pattern for video (ffmpeg CLI to virtual camera)
- Phase 3 wires frontend to call these commands
