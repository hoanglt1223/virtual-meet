# VirtualMeet Technical Architecture

## System Overview

VirtualMeet is a Windows desktop application that processes media files and presents them as virtual webcam and microphone devices to online meeting applications.

## Architecture Layers

```
┌─────────────────────────────────────────┐
│            Frontend UI Layer            │
│  React + Vite + Tailwind + shadcn/ui   │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│         Tauri Bridge Layer              │
│    Command API + State Management       │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│           Rust Core Layer               │
│   Media Processing + Device Control     │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│        Platform Integration             │
│     Windows APIs + Device Drivers       │
└─────────────────────────────────────────┘
```

## Core Components

### 1. Media Pipeline Engine (Rust)

**Video Pipeline:**
- MP4 decoding using FFmpeg bindings
- Frame buffering and format conversion
- DirectShow/Media Foundation integration for virtual webcam output
- Real-time frame delivery to virtual device

**Audio Pipeline:**
- MP3/PCM decoding using rodio/FFmpeg
- Audio resampling and format conversion
- WASAPI integration for virtual microphone output
- Volume control and mixing

**Recording Pipeline:**
- Simultaneous video + audio capture
- Real-time muxing to MP4 format
- Configurable quality presets
- Timestamp management

### 2. Device Management (Rust)

**Virtual Device Discovery:**
- Enumerate available webcam devices
- Identify virtual vs physical devices
- Enumerate audio input/output devices
- Device capability detection

**Device Control:**
- Virtual webcam device initialization
- Virtual audio device initialization
- Stream routing and configuration
- Device state management

### 3. Automation Engine (Rust)

**Scripting System:**
- JSON/DSL parsing and validation
- Script execution runtime
- Sequential and parallel command processing
- Error handling and recovery

**Hotkey System:**
- Global hotkey registration
- Hotkey to media/action mapping
- Cross-process hotkey delivery
- Hotkey conflict detection

### 4. Frontend Architecture (React)

**Component Structure:**
```
App
├── Dashboard (preview + controls)
├── MediaLibrary (file browser + thumbnails)
├── Recording (record controls + file list)
└── Settings (device config + preferences)
```

**State Management:**
- Tauri invoke for all core operations
- Local UI state for presentation
- Reactive updates from Rust backend

## Data Flow

### Video Playback Flow:
```
MP4 File → FFmpeg Decoder → Frame Buffer → Format Converter → Virtual Webcam → Meeting App
```

### Audio Playback Flow:
```
MP3 File → Audio Decoder → Resampler → WASAPI → Virtual Mic → Meeting App
```

### Recording Flow:
```
Current Video + Audio → Frame/Audio Capture → Real-time Muxer → MP4 File
```

### Hotkey Flow:
```
Global Hotkey → Hotkey Manager → Script Engine → Media Pipeline → Virtual Devices
```

## Key Technical Decisions

### 1. Rust Core for All Media Processing
- **Rationale**: Performance, memory safety, and real-time guarantees
- **Benefits**: No garbage collection pauses, deterministic resource management
- **Trade-offs**: Steeper learning curve, more verbose code

### 2. FFmpeg for Media Decoding
- **Rationale**: Broad format support, optimized performance
- **Benefits**: Hardware acceleration support, proven reliability
- **Trade-offs**: Large dependency, complex build

### 3. Windows-specific APIs for Device Integration
- **Rationale**: Direct access to virtual device capabilities
- **Benefits**: Maximum compatibility, lowest latency
- **Trade-offs**: Windows-only deployment

### 4. Tauri for Desktop App Framework
- **Rationale**: Rust integration, small bundle size, web-based UI
- **Benefits**: Cross-platform potential, rapid UI development
- **Trade-offs**: Learning curve for Tauri-specific patterns

## Performance Requirements

### Real-time Constraints:
- **Video**: 30fps delivery to virtual webcam
- **Audio**: 48kHz delivery to virtual microphone
- **Switching**: <200ms media clip changes
- **Startup**: <300ms media library loading (100+ files)

### Memory Management:
- **Frame Buffer**: 3-5 frames of video per active stream
- **Audio Buffer**: 100-200ms of audio per active stream
- **Metadata**: In-memory media library index
- **Recording**: Efficient file I/O with minimal buffering

### CPU Usage Targets:
- **Idle**: <5% CPU usage
- **Active Playback**: 10-20% CPU usage (single stream)
- **Recording**: Additional 5-10% CPU usage
- **Peak**: <50% CPU usage under load

## Security Considerations

### File System Access:
- User-selected media folders only
- No automatic file deletion/modification
- Validate file formats before processing

### Device Integration:
- Read-only device enumeration
- No modification of system drivers
- Isolated virtual device creation

### Script Execution:
- Sandboxed scripting environment
- No file system access from scripts
- Limited API surface for scripts

## Error Handling Strategy

### Media Processing Errors:
- Graceful fallback on format issues
- User notification with actionable error messages
- Automatic retry for transient failures

### Device Errors:
- Clear indication of device availability
- Automatic device fallback options
- Recovery procedures for device disconnection

### System Resource Errors:
- Memory usage monitoring and limits
- Disk space validation for recording
- CPU usage throttling under load

## Development Phases

### Phase 1: Foundation
- Project structure and build system
- Basic Tauri + React integration
- Core Rust workspace setup

### Phase 2: Media Processing
- Video pipeline implementation
- Audio pipeline implementation
- Device enumeration and control

### Phase 3: User Interface
- Complete React UI components
- Tauri command API implementation
- State management integration

### Phase 4: Advanced Features
- Recording functionality
- Scripting engine
- Hotkey system

### Phase 5: Polish & Testing
- Performance optimization
- Error handling
- User experience refinement