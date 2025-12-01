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

### Virtual Device Integration ✅ COMPLETED

Comprehensive virtual device integration using Windows APIs has been implemented with the following architecture:

#### Virtual Webcam Implementation

**DirectShow Backend (`DirectShowVirtualWebcam`)**:
- Filter graph management with `IMediaControl` and `IBaseFilter`
- Virtual source filter creation and video renderer setup
- Frame delivery through DirectShow media samples
- Format negotiation and pin connection management
- COM initialization and cleanup

**Media Foundation Backend (`MediaFoundationVirtualWebcam`)**:
- Media session management with `IMFMediaSession`
- Presentation descriptor handling
- Custom `IMFMediaSource` implementation framework
- Sample delivery through Media Foundation pipeline
- MF startup/shutdown management

**Core Virtual Webcam (`VirtualWebcam`)**:
- Backend abstraction with configurable DirectShow/Media Foundation selection
- FFmpeg-based video decoding with frame buffer management
- Thread-safe playback loop with real-time frame delivery
- Video format conversion (RGB24) and frame timing synchronization
- Buffer management with configurable capacity and overflow handling

#### Virtual Microphone Implementation

**WASAPI Backend (`WasapiVirtualMicrophone`)**:
- Audio client initialization with `IAudioClient`
- Device enumeration through `IMMDeviceEnumerator`
- Audio format configuration (WAVEFORMATEX)
- Loopback mode setup for virtual microphone functionality
- Audio endpoint management and COM initialization

**Kernel Streaming Backend (`KSVirtualMicrophone`)**:
- KS filter interface foundation
- Audio driver integration framework
- Low-level audio sample delivery
- Pin and topology management structure

**Core Virtual Microphone (`VirtualMicrophone`)**:
- Backend abstraction with WASAPI/Kernel Streaming options
- Audio pipeline integration with decoding and processing
- Volume control and mute functionality
- Real-time audio sample delivery to virtual endpoints
- Visualization and processing statistics

#### Media Router System (`MediaRouter`)

**Synchronization Features**:
- Audio/video synchronization with timestamp tracking
- Sync offset calculation and correction
- Background synchronization thread
- Configurable sync intervals and tolerance levels

**Media Management**:
- Concurrent audio/video streaming coordination
- Runtime media file switching without interruption
- Loop playback and format configuration
- Volume control for both audio and video streams

**Configuration System**:
- Flexible backend selection for both devices
- Runtime configuration updates
- Status monitoring and reporting
- Error handling and recovery mechanisms

#### Device Enumeration Enhancements

**DirectShow Device Discovery**:
- System device enumerator integration
- Video input device enumeration with friendly names
- Moniker-based device identification
- Capture device capability detection

**Media Foundation Device Discovery**:
- Attribute-based device filtering
- Audio capture endpoint enumeration
- Device activation and property retrieval
- Friendly name extraction

**WASAPI Device Discovery**:
- MM device enumerator integration
- Audio endpoint enumeration (capture/render)
- Device state filtering
- ID-based device identification

#### Tauri Commands and API

**Virtual Device Commands**:
- `initialize_webcam(backend)` - Initialize virtual webcam with specified backend
- `initialize_microphone(backend)` - Initialize virtual microphone with specified backend
- `start_webcam_streaming(video_path)` - Start video streaming to virtual webcam
- `start_microphone_streaming(audio_path)` - Start audio streaming to virtual microphone
- `stop_webcam_streaming()` - Stop virtual webcam streaming
- `stop_microphone_streaming()` - Stop virtual microphone streaming
- `get_virtual_device_status()` - Get comprehensive device status
- `list_virtual_devices()` - Enumerate available virtual and physical devices

**Media Router Commands**:
- `initialize_media_router(video_backend, audio_backend)` - Initialize media router
- `start_media_routing(video_path, audio_path, sync, loop, volumes)` - Start coordinated media playback
- `stop_media_routing()` - Stop media routing and cleanup
- `switch_media(video_path, audio_path)` - Switch media files during playback
- `get_media_routing_status()` - Get detailed routing status and statistics

**Control Commands**:
- `set_microphone_volume(volume)` - Set microphone volume (0.0-1.0)
- `get_microphone_volume()` - Get current microphone volume
- `set_microphone_muted(muted)` - Set microphone mute state
- `get_microphone_muted()` - Get microphone mute state
- `get_webcam_video_info()` - Get current video information
- `get_webcam_buffer_status()` - Get frame buffer statistics
- `get_microphone_buffer_status()` - Get audio buffer statistics

#### Testing Framework

**Unit Tests**:
- Virtual device creation and backend selection
- Initialization and cleanup procedures
- Error handling and edge case scenarios
- Configuration management and updates
- Device enumeration and filtering

**Integration Tests**:
- Media router lifecycle management
- Concurrent device access patterns
- Real-world media file handling
- Performance under load conditions
- Memory leak detection and cleanup

**Mock Testing**:
- Temporary file creation for media testing
- Invalid media file handling
- Concurrent access validation
- Error recovery mechanisms

#### Implementation Architecture

**File Structure**:
```
src-tauri/src/virtual/
├── mod.rs              # Module exports and test integration
├── webcam.rs           # Virtual webcam implementation
├── microphone.rs       # Virtual microphone implementation
├── media_router.rs     # Media coordination and synchronization
└── tests.rs           # Comprehensive test suite

src-tauri/src/commands/
└── virtual_devices.rs # Tauri API commands for virtual devices
```

**Key Dependencies**:
- `windows-rs`: DirectShow, Media Foundation, WASAPI, and COM APIs
- `ffmpeg-next`: Video decoding and format conversion
- `cpal`: Cross-platform audio library integration
- `tokio`: Async runtime and concurrency management
- `anyhow`: Error handling and result management
- `tracing`: Comprehensive logging and debugging

**Backend Support**:
- **Webcam**: DirectShow and Media Foundation
- **Microphone**: WASAPI and Kernel Streaming
- **Audio Processing**: CPAL integration with format conversion
- **Video Processing**: FFmpeg decoding with real-time scaling

#### Performance Characteristics

**Latency Optimization**:
- Direct frame delivery without intermediate storage
- Minimal buffer utilization with overflow management
- Real-time format conversion and processing
- Thread-safe operations with lock-free data structures

**Memory Management**:
- Configurable buffer sizes for video frames and audio samples
- Automatic cleanup and resource recovery
- Overflow handling with frame dropping strategies
- Memory leak prevention through RAII patterns

**CPU Utilization**:
- Efficient video decoding with hardware acceleration support
- Background threading for non-blocking operations
- Configurable processing loads and quality settings
- Adaptive sync intervals based on system performance

#### Error Handling and Recovery

**Comprehensive Error Management**:
- Graceful degradation when virtual device drivers are unavailable
- Automatic fallback to simulation modes for testing
- Detailed error reporting through Tauri command responses
- Recovery mechanisms for device disconnection

**Logging and Debugging**:
- Structured logging with tracing integration
- Performance metrics and status reporting
- Debug information for device enumeration failures
- Sync state monitoring and reporting

#### Files Added/Modified

**Core Implementation**:
- `src-tauri/src/virtual/webcam.rs` - Complete virtual webcam implementation
- `src-tauri/src/virtual/microphone.rs` - Complete virtual microphone implementation
- `src-tauri/src/virtual/media_router.rs` - Media coordination and synchronization system
- `src-tauri/src/virtual/mod.rs` - Module exports and integration
- `src-tauri/src/virtual/tests.rs` - Comprehensive test suite

**Tauri Integration**:
- `src-tauri/src/commands/virtual_devices.rs` - Virtual device API commands
- `src-tauri/src/commands.rs` - Updated with virtual device exports
- `src-tauri/Cargo.toml` - Added Windows API dependencies

**Documentation**:
- Enhanced `docs/todo.md` with complete implementation details
- Integration examples and usage patterns
- Performance analysis and optimization details
- Testing strategies and validation procedures

## Next Steps
1. Develop frontend UI for virtual device management and control
2. Implement hotkey system for media switching and control
3. Create installer and distribution packaging
4. Add comprehensive user documentation and tutorials
5. Performance testing and optimization on various Windows configurations
6. Security audit and virtual device driver certification