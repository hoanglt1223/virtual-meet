# VirtualMeet - Project Requirements & Tasks

## Project Overview
**VirtualMeet** is an open-source Windows application that simulates virtual presence by outputting pre-recorded video/audio as fake webcam and microphone inputs for meeting applications.

## Business Requirements

### Core Features
- **Virtual Webcam**: Play MP4 files as webcam output with looping, fast switching, playlists, and hotkey triggers
- **Virtual Microphone**: Play MP3 files as microphone output with looping, fast switching, playlists, and volume control
- **Recording**: Record combined video + audio to MP4 for content creation and reuse
- **Media Library**: Scan folders, display thumbnails, quick file selection
- **Automation**: Simple scripting (JSON/DSL) to chain clips or trigger sequences by hotkey
- **Settings**: Configure virtual devices, media folders, recording quality, hotkeys

### Technical Requirements
- **Rust Core**: High-performance decode/encode and media pipeline
- **Windows Integration**: WASAPI audio, DirectShow/MF video output
- **Virtual Device Support**: Compatible with existing virtual webcam/microphone drivers

## Acceptance Criteria

1. MP4 plays smoothly as virtual webcam in Zoom/Meet/Teams
2. MP3 plays as microphone input in any app using virtual audio drivers
3. Video/audio switching feels instant (<200ms)
4. Recording produces clean MP4 with correct A/V sync
5. Media library loads fast even with 100+ files
6. Hotkeys and scripts trigger reliably
7. App doesn't crash under rapid switching
8. All settings persist

## Technical Architecture

### Core Components
```
VirtualMeet/
├── core/                 # Rust core library
│   ├── media_pipeline/   # Video/audio processing
│   ├── virtual_devices/  # Virtual webcam/mic integration
│   └── automation/       # Scripting engine
├── ui/                   # Frontend (Tauri + egui)
├── drivers/              # Virtual device interfaces
└── packaging/            # Installer and distribution
```

### Dependencies
- **Rust**: Core media processing and performance
- **Tauri**: Cross-platform desktop app framework
- **egui**: Immediate mode GUI framework
- **ffmpeg-next**: Video/audio decoding and encoding
- **windows-rs**: Windows API integration
- **serde**: JSON serialization for settings and scripts

## Implementation Tasks

📋 **See detailed kanban board in [kanban-board.md](./kanban-board.md)**

### Sprint Overview (8 Weeks)
- **Week 1**: Foundation & Core Infrastructure (10 tasks)
- **Week 2**: Virtual Devices & Recording (8 tasks)
- **Week 3**: Tauri Bridge & Frontend Setup (5 tasks)
- **Week 4**: Core UI Development (6 tasks)
- **Week 5**: Advanced Features (7 tasks)
- **Week 6**: Platform Integration (4 tasks)
- **Week 7**: Testing & Quality Assurance (5 tasks)
- **Week 8**: Documentation & Release (5 tasks)

### Phase 1: Core Infrastructure ✅ COMPLETED
- [x] Project setup and documentation
- [x] Rust workspace initialization
- [x] Core media pipeline structure
- [x] Device enumeration system implementation
- [x] Virtual device detection and integration
- [ ] Basic UI framework setup

### Phase 2: Media Playback
- [ ] MP4 video playback implementation
- [ ] MP3 audio playback implementation
- [ ] Media file scanning and library
- [ ] Thumbnail generation for videos

### Phase 3: Virtual Output
- [ ] Virtual webcam driver integration
- [ ] Virtual microphone driver integration
- [ ] Real-time video streaming to virtual devices
- [ ] Real-time audio streaming to virtual devices

### Phase 4: User Interface
- [ ] Media library UI with thumbnails
- [ ] Playback controls and switching UI
- [ ] Settings panel implementation
- [ ] Hotkey configuration system

### Phase 5: Advanced Features
- [ ] Recording functionality (combined A/V)
- [ ] Playlist management
- [ ] Scripting engine implementation
- [ ] Automation and hotkey triggers

### Phase 6: Polish & Release
- [ ] Error handling and stability
- [ ] Performance optimization
- [ ] Installer creation
- [ ] Documentation and README
- [ ] MIT license application

## Success Metrics
- Video/audio switching latency <200ms
- Support for 100+ media files in library
- Stable operation under rapid switching
- Compatible with major meeting platforms (Zoom, Teams, Meet)
- Clean A/V sync in recordings

## Completed Major Implementations

### Device Enumeration System ✅ COMPLETED

The comprehensive device enumeration system has been successfully implemented with the following components:

#### Core Features
- **Audio Device Discovery**: WASAPI and CPAL integration for complete audio device enumeration
- **Video Device Discovery**: DirectShow and Media Foundation integration for webcam enumeration
- **Virtual vs Physical Detection**: Advanced pattern matching and manufacturer identification
- **Device Capability Detection**: Comprehensive format, resolution, and performance testing
- **Performance Optimization**: Caching, async operations, and parallel processing

#### Virtual Device Support
- **Audio Virtual Devices**: VB-CABLE, VoiceMeeter, OBS Virtual Audio, and more
- **Video Virtual Devices**: OBS Virtual Camera, SplitCam, ManyCam, Snap Camera, and more
- **Manufacturer Detection**: Realtek, Logitech, Microsoft, NVIDIA, AMD, Intel identification
- **Driver Analysis**: Software vs hardware driver classification

#### Tauri API Commands
- `enumerate_all_devices()` - Complete device enumeration with filtering
- `enumerate_audio_devices()` - Audio-specific device discovery
- `enumerate_video_devices()` - Video-specific device discovery
- `get_device_capabilities()` - Detailed device capability detection
- `is_device_virtual()` - Virtual/physical status checking
- `get_virtual_devices()` - Virtual devices only filtering
- `get_physical_devices()` - Physical devices only filtering
- `refresh_device_list()` - Device list refresh functionality

#### Documentation
- **Implementation Guide**: `DEVICE_ENUMERATION_IMPLEMENTATION.md`
- **API Documentation**: Comprehensive command and data structure documentation
- **Testing Examples**: Unit tests and integration test examples
- **Performance Analysis**: Memory, CPU, and latency optimization details

#### Files Added/Modified
- `src-tauri/src/devices.rs` - Main device enumeration system
- `src-tauri/src/devices/audio.rs` - Audio device enumerator
- `src-tauri/src/devices/video.rs` - Video device enumerator
- `src-tauri/src/devices/capabilities.rs` - Device capability detector
- `src-tauri/src/commands.rs` - Updated with device enumeration commands
- `src-tauri/Cargo.toml` - Added chrono dependency
- `DEVICE_ENUMERATION_IMPLEMENTATION.md` - Complete implementation documentation

## Next Steps
1. Set up basic Tauri + React application structure
2. Begin frontend development with device management UI
3. Implement virtual device integration with existing pipelines
4. Add real-time device monitoring and hot-plug detection