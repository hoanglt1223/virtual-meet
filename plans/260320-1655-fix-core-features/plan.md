---
title: "Fix Core Audio/Video Streaming & Frontend Wiring"
description: "Make audio playback to selected device, video streaming via ffmpeg CLI, and Dashboard IPC actually work"
status: completed
priority: P1
effort: 12h
branch: main
tags: [audio, video, frontend, bugfix, core]
created: 2026-03-20
---

# Fix Core Features Plan

## Problem Summary

1. **Audio streaming stubbed** -- `start_audio_streaming` command returns TODO failure; `AppState` wraps `VirtualMicrophone` in `Arc` (immutable) but `start_streaming` needs `&mut self`
2. **Webcam backends all stubs** -- DirectShow/MediaFoundation `send_frame` are TODOs; the code tries to CREATE virtual drivers from userspace (impossible without kernel dev)
3. **Dashboard has zero IPC** -- all state is mock, no `invoke()` calls, no file picker
4. **playback_loop signature mismatch** -- takes both DS+MF backends but caller passes single `backend` Arc (compile error or wrong type)

## Architecture Decision

**Audio**: Abandon the fake WASAPI virtual microphone approach. Instead, use rodio/cpal to play audio to a **user-selected output device**. If user picks "CABLE Input (VB-Audio Virtual Cable)" as output, meeting apps see it as mic input. This is just audio playback to a specific device -- no kernel driver needed.

**Video**: Abandon DirectShow/MediaFoundation userspace virtual driver creation. Instead, use `ffmpeg` CLI (already a required dependency) to pipe decoded frames to an existing virtual camera. Command: `ffmpeg -re -stream_loop -1 -i input.mp4 -f dshow -video_size 1280x720 -vcodec rawvideo -pixel_format yuyv422 "video=OBS Virtual Camera"`. Simplest approach; we already require ffmpeg for the ffmpeg-next crate.

**Frontend**: Wire Dashboard to Tauri `invoke()` for file selection (dialog plugin), streaming start/stop, volume, and device listing.

## Phases

| # | Phase | Status | Effort |
|---|-------|--------|--------|
| 1 | [Audio playback to selected device](phase-01-audio-playback.md) | completed | 4h |
| 2 | [Video streaming via ffmpeg CLI](phase-02-video-streaming.md) | completed | 4h |
| 3 | [Dashboard frontend IPC wiring](phase-03-frontend-wiring.md) | completed | 4h |

## Key Dependencies

- Phase 2 and 3 can be done in parallel after Phase 1
- Phase 3 depends on Phase 1 and 2 command signatures being finalized
- OBS Virtual Camera driver must be installed on user machine for video
- VB-CABLE or VoiceMeeter must be installed for audio-as-mic

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| ffmpeg CLI not in PATH | Video won't work | Check at startup, use `custom_ffmpeg_path` from settings |
| OBS Virtual Camera not installed | Video won't work | Show clear error message, document requirement |
| cpal device enumeration missing VB-CABLE | Audio-as-mic won't work | Fall back to default output; document requirement |
| Existing tests break from refactor | CI fails | Run `cargo test` and `pnpm test` after each phase |
