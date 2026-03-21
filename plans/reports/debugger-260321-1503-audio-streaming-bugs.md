# Audio Streaming Bug Report
Date: 2026-03-21 | Investigator: debugger

---

## Executive Summary

6 code paths analyzed. Found **5 bugs** of varying severity: 1 critical, 2 medium, 2 low. No critical IPC deserialization mismatch found (contrary to hypothesis), but several logic, data-safety, and feature-gap issues exist.

---

## Bug Inventory

### BUG-01 [CRITICAL] — cpal callback calls `pop_frame()` on `&mut self`, but `AudioBuffer` is behind `Arc<StdMutex<AudioBuffer>>`

**File:** `src-tauri/src/audio.rs` lines 160–163
**File:** `src-tauri/src/virtual_device/microphone.rs` lines 188, 206

`AudioBuffer::pop_frame()` and `push_frame()` both require `&mut self`. But in `VirtualMicrophone::build_stream`, the `buffer` captured by the closure is `Arc<StdMutex<AudioBuffer>>`. The cpal callback does:

```rust
let mut buf = match buffer.lock() { Ok(b) => b, ... };
if let Some(frame) = buf.pop_frame() {       // line 188
    ...
    let _ = buf.push_frame(frame);           // line 206 — re-adds for looping
}
```

`buf` is a `MutexGuard<AudioBuffer>`, and `pop_frame`/`push_frame` are `&mut self` methods. This is fine via `DerefMut`, BUT — the inner `VecDeque` itself is also wrapped in `Arc<StdMutex<VecDeque<...>>>` inside `AudioBuffer` (audio.rs line 123). This creates **double-locking**: the outer `StdMutex<AudioBuffer>` is locked, then `pop_frame` tries to lock the inner `Arc<StdMutex<VecDeque>>`. In a single-threaded callback context this won't deadlock, but if `audio_buffer` is simultaneously accessed from `start_streaming` (which calls `buffer.push_frame(frame)` after locking the outer mutex at line 107), there is a **lock ordering inconsistency**: `start_streaming` holds the outer mutex while calling `push_frame` which tries the inner mutex; the cpal callback also holds the outer mutex while calling `pop_frame` which tries the inner mutex. Both run in different threads — no deadlock since they're sequential under the outer mutex, but the inner `Arc<StdMutex<VecDeque>>` inside `AudioBuffer` is entirely redundant and wasteful. More importantly: **`get_buffer_status` (line 289 in microphone.rs) calls `buffer.lock().unwrap()` without `.map_err`** — will panic on lock poison.

**Severity:** Critical potential panic; structural redundancy

---

### BUG-02 [MEDIUM] — `decode_all` in `audio_decoder.rs` silently drops `SymphoniaError::ResetRequired` without re-opening the format reader

**File:** `src-tauri/src/audio_decoder.rs` lines 142–145

```rust
Err(SymphoniaError::ResetRequired) => {
    warn!("Decoder reset required");
    continue;  // ← tries to read next packet but decoder state is invalid
}
```

When symphonia emits `ResetRequired`, the decoder must be reset via `decoder.reset()` before the next `decode()` call, **otherwise subsequent `decode()` calls return garbage or error**. The current code just `continue`s the packet-read loop without calling `decoder.reset()`. For some codecs (e.g., gapless MP3) this is hit on every new "segment" — result is that frames after the reset point are silently dropped or produce decode errors which then `break` the loop early (line 190–193), truncating the audio.

**Severity:** Medium — audio file decodes partially for some formats

---

### BUG-03 [MEDIUM] — Volume/mute controls have no effect during cpal playback

**File:** `src-tauri/src/virtual_device/microphone.rs` lines 162–218

The cpal stream callback in `build_stream` reads directly from `AudioBuffer` frames and converts them to output samples **without consulting `AudioProcessor`**. Volume (`set_volume`) and mute (`set_muted`, `toggle_mute`) modify `AudioProcessor` state, but the callback never calls `AudioProcessor::process_frame`. Thus the volume slider and mute button in `Dashboard.tsx` (`handleVolumeChange`, `handleMuteToggle`) invoke the Tauri commands which update `AudioProcessor`, but the actual audio stream output is unaffected.

**Severity:** Medium — volume/mute feature silently broken

---

### BUG-04 [LOW] — `open()` method in `AudioDecoder` decodes track info but discards `track_id` and format reader; `decode_all` re-opens the file

**File:** `src-tauri/src/audio_decoder.rs` lines 40–84, 87–199

`open()` opens the file, extracts metadata into `self.metadata`, but drops the `format` reader and `track_id` — they are local variables. `decode_all()` re-opens the file independently. This means:
- In `validate_audio_file` (commands.rs line 561–563), `decoder.open(&path)` is called — which works correctly for validation.
- In `start_streaming` (microphone.rs line 97), `decoder.decode_all(...)` is called — which **also works** but re-probes the file from scratch.
- `current_position` field in `AudioDecoder` is set in `set_position()` but never updated in `decode_all`, making `get_position()` always return 0.

**Severity:** Low — no functional break, but misleading API; position tracking non-functional

---

### BUG-05 [LOW] — `setup-panel.tsx` calls `window.open(rec.action_url!, "_blank")` which is blocked in Tauri v2 without `shell:allow-open` configured for URLs

**File:** `src/components/setup-panel.tsx` line 186

```tsx
onClick={() => window.open(rec.action_url!, "_blank")}
```

Tauri v2 intercepts `window.open` calls. The capability file (`src-tauri/capabilities/default.json`) includes `shell:allow-open` which enables the shell plugin's open command, but `window.open` in the WebView is a different path — it goes through Tauri's navigation handler. In Tauri v2, external URL navigation from WebView requires explicit CSP allowance or `openWith` plugin. The `tauri.conf.json` has `"csp": null` which disables CSP enforcement, so this _may_ work, but behavior is inconsistent across Tauri v2 versions. The correct approach is `invoke("plugin:shell|open", { path: url })` or `import { open } from "@tauri-apps/plugin-shell"`.

**Severity:** Low — install links may silently fail depending on Tauri v2 runtime

---

## IPC Deserialization: CONFIRMED CORRECT

The specific concern raised in the investigation request was:

> Frontend sends `{ request: { path: audioPath } }` — does this match Rust?

**Analysis:** `Dashboard.tsx` line 220:
```ts
invoke("start_audio_streaming", { request: { path: audioPath } })
```

Rust command signature (`commands.rs` line 399–402):
```rust
pub async fn start_audio_streaming(
    request: AudioRequest,
    state: State<'_, AppState>,
) -> Result<VideoResponse, String>
```

Where `AudioRequest` is:
```rust
pub struct AudioRequest { pub path: String }
```

In Tauri v2, each top-level key in the invoke args object maps to a named parameter. `{ request: { path: audioPath } }` → `request: AudioRequest { path: String }`. **This is correct.** No deserialization bug here.

---

## Recording Commands: `Mutex` Deadlock Risk

**File:** `src-tauri/src/commands_recording.rs` lines 26–29, 53–56, 73–76, 104–107

All recording commands (`start_recording`, `stop_recording`, `get_recording_status`, `update_recording_config`) call `app.state::<std::sync::Mutex<CombinedRecorder>>().lock()` inside `async` functions. This holds a `std::sync::MutexGuard` across what may be async await points — but in this code the guards are dropped before any `.await` calls, so no deadlock risk exists here. However, **`get_recording_status` at line 79–88 makes three separate lock/unlock cycles** (`get_state`, `get_stats`, `get_current_session`), each re-locking. If `CombinedRecorder` is modified between these calls (unlikely since it's single-threaded), results could be inconsistent. Low risk but noted.

---

## Summary Table

| # | Severity | File | Line | Issue |
|---|----------|------|------|-------|
| 01 | Critical | `audio.rs` / `microphone.rs` | 123, 289 | `get_buffer_status` unwrap panics on poison; redundant double-mutex |
| 02 | Medium | `audio_decoder.rs` | 142–145 | `ResetRequired` not handled — decoder not reset before next decode |
| 03 | Medium | `microphone.rs` | 162–218 | cpal callback ignores `AudioProcessor` — volume/mute have no effect |
| 04 | Low | `audio_decoder.rs` | 40–84 | `open()` discards state; position tracking always returns 0 |
| 05 | Low | `setup-panel.tsx` | 186 | `window.open` may be blocked in Tauri v2 for external URLs |

---

## Unresolved Questions

1. What does `CombinedRecorder::get_state()` / `get_stats()` / `get_current_session()` return when recorder failed to initialize? (recorder init failure at main.rs line 92 is logged but not fatal — subsequent command calls would panic on `app.state::<Mutex<CombinedRecorder>>()` if state was never managed.)
2. Does `AudioBuffer::max_size` of 8192 (from `AudioConfig::default()`) mean only 8192 _frames_ are buffered? For a typical 44100 Hz stereo file at ~1000 frames/sec that is ~8 seconds of audio — after that frames are silently dropped via `pop_front` at `audio.rs:145`. Is this intentional (streaming window) or a bug?
3. Is `BufReader` wrapping intentional in `audio.rs`? The `MediaSourceStream` at `audio_decoder.rs:99` wraps a raw `File` — adding `BufReader` would improve read performance for large files.
