# Planner Report: Fix Core Audio/Video Streaming & Frontend Wiring

**Plan**: `D:\Projects\virtual-meet\plans\260320-1655-fix-core-features\plan.md`
**Date**: 2026-03-20

## Summary

Created 3-phase plan to make VirtualMeet's core features actually work. Root cause across all issues: the codebase tries to create virtual device **drivers** from userspace (impossible without kernel development). Solution: use existing virtual device drivers (OBS Virtual Camera, VB-CABLE) as output targets.

## Key Architectural Decisions

1. **Audio**: Drop WASAPI/KS virtual mic creation. Instead, play decoded audio to a user-selected cpal output device. If user picks "CABLE Input", meeting apps see it as mic. This is just audio playback -- no driver needed.

2. **Video**: Drop DirectShow/MediaFoundation virtual webcam creation. Instead, spawn `ffmpeg` CLI subprocess to stream video to OBS Virtual Camera via `-f dshow`. ffmpeg handles decode, timing, format conversion. ~20 lines of Rust replaces ~400 lines of stub code.

3. **Frontend**: Wire Dashboard.tsx to actual `invoke()` calls. Add Tauri dialog plugin file picker, device selector dropdowns, error display.

## Phase Breakdown

| Phase | What | Core Change | Effort |
|-------|------|-------------|--------|
| 1 - Audio | Play audio to selected output device | `AppState.microphone` -> `Arc<Mutex<VirtualMicrophone>>`, add device selection to cpal stream | 4h |
| 2 - Video | ffmpeg CLI subprocess to virtual camera | New `VideoStreamState` with `Child` process handle, `start_video_streaming`/`stop_video_streaming` commands | 4h |
| 3 - Frontend | Dashboard IPC wiring | `invoke()` calls, dialog file picker, device selectors, remove mock state | 4h |

## Bugs Found During Analysis

- **playback_loop signature mismatch** (`webcam.rs:989` vs `:813`): Function takes 7 params (both DS+MF backends) but caller passes single `backend` Arc. This is a compile error in the current code. Phase 2 obsoletes this code.
- **tokio Runtime inside cpal callback** (`microphone.rs:660`): Creates a new tokio Runtime per audio frame callback to call async WASAPI backend. This would cause severe performance issues. Phase 1 removes this.
- **`commands.rs` duplicate command names**: Both `commands::set_microphone_volume` and `commands::virtual_devices::set_microphone_volume` registered in handler. May cause conflicts. Phase 1 should consolidate.

## Unresolved Questions

1. **OBS Virtual Camera ffmpeg output format**: Does OBS VC accept raw video via `-f dshow` directly, or does it need specific pixel format flags (`-vcodec rawvideo -pix_fmt yuyv422`)? Needs testing on actual machine with OBS installed.
2. **ffmpeg device listing parse format**: The stderr output from `ffmpeg -list_devices true -f dshow -i dummy` varies across ffmpeg versions. May need robust regex or alternative enumeration method.
3. **Should old VirtualWebcam/VirtualMicrophone code be deleted or kept?** Plan currently marks as `#[allow(dead_code)]`. Could delete to reduce maintenance burden, but might want for future kernel driver integration.
