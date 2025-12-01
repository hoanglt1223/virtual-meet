# VirtualMeet Requirements

## Business Requirements

### Virtual Webcam (Video)
- Play MP4 as webcam output
- Loop video seamlessly
- Fast switch between videos (<200ms)
- Simple playlists support
- Hotkey triggers for specific clips
- Local preview window
- Output to virtual webcam device (Zoom/Meet/Teams compatible)

### Virtual Microphone (Audio)
- Play MP3 and other common audio formats as microphone input
- Loop audio seamlessly
- Fast switch between audio clips (<200ms)
- Basic playlist support
- Volume control and mute functionality
- Output to virtual mic/audio device

### Recording
- Record combined current video + audio to MP4
- Configurable resolution (720p/1080p) and quality presets
- Timestamped filenames
- Start/stop controls with status indicators

### Media Library / UX
- Scan one or more folders for MP4/MP3 files
- Display thumbnails and basic metadata
- One-click "Set as current video/audio"
- Quick search and filtering
- Fast loading for 100+ files (<300ms on SSD)

### Automation & Hotkeys
- Simple JSON/DSL scripting for sequences
- Bind scripts or clips to global hotkeys (F1, F2, etc.)
- Script execution control (start, stop, cancel)
- Works when app window is not focused

### Settings
- Configure media folders
- Choose virtual webcam and mic devices
- Configure recording defaults
- Set up global hotkeys
- Persistent settings across restarts

## Technical Requirements

### Rust Core (All business logic)
- Video decode and playback pipeline
- Audio decode and playback pipeline
- Device enumeration and management
- Recording pipeline (muxing video+audio)
- Scripting engine and runtime
- Global hotkey handling
- Tauri command API for frontend

### Frontend (Thin UI layer)
- Main dashboard with preview and controls
- Media library with thumbnails
- Recording interface
- Settings panels
- Hotkey configuration

### Platform Integration
- WASAPI for audio device management
- DirectShow/Media Foundation for webcam devices
- Proper device routing for virtual drivers

## Acceptance Criteria

1. Selected MP4 loops smoothly as webcam in meeting apps
2. Selected MP3 plays as microphone through virtual device
3. Video/audio switching feels instant (<200ms perceived delay)
4. Recording produces sync'd, glitch-free MP4 files
5. Media library loads 100+ files quickly (<300ms)
6. Hotkeys work reliably when app is unfocused
7. App remains stable under rapid switching/recordings
8. User settings persist between sessions
9. All core logic is in Rust (JS is UI only)
10. MIT licensed, excluding virtual drivers

## Constraints

- Target platform: Windows only
- No kernel driver development (use existing virtual devices)
- Rust core must handle all media processing
- Frontend must be thin UI layer only
- Must work with popular meeting apps