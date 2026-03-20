# VirtualMeet

An open-source Windows desktop app that sends pre-recorded video & audio into online meeting apps as a virtual webcam and virtual microphone. Built with Tauri v2 (React + Rust).

## Features

- **Virtual Webcam** — Stream video files to virtual camera (OBS Virtual Camera or built-in IMFVirtualCamera on Windows 11)
- **Virtual Microphone** — Play audio files through a virtual audio cable (VB-CABLE/VoiceMeeter) so meeting apps see it as mic input
- **Dual Webcam Backend** — OBS mode (ffmpeg CLI) or built-in mode (Windows 11 IMFVirtualCamera, no OBS needed)
- **Device Auto-Detection** — Automatically detects installed virtual devices and guides setup
- **Media Library** — Browse, search, and manage media files with thumbnails
- **Recording** — Record combined video + audio to MP4
- **Global Hotkeys** — Quick media switching without leaving your meeting
- **Scripting** — Rhai scripting engine + JSON DSL for automation

## How It Works

```
Video file ──→ ffmpeg CLI ──→ OBS Virtual Camera ──→ Zoom/Teams sees "webcam"
                  OR
Video file ──→ Shared Memory ──→ vcam_source.dll (COM) ──→ Windows Frame Server ──→ "VirtualMeet Camera"

Audio file ──→ cpal playback ──→ VB-CABLE Input ──→ Zoom/Teams sees "microphone"
```

## Requirements

| Component | Required | Purpose |
|-----------|----------|---------|
| Windows 10+ | Yes | OS |
| FFmpeg | Yes (OBS mode) | Video decoding & streaming |
| OBS Virtual Camera | OBS mode | Virtual webcam device |
| Windows 11 build 22000+ | IMF mode | Built-in virtual camera (no OBS) |
| VB-CABLE or VoiceMeeter | Yes | Virtual audio cable for mic routing |

## Quick Start

1. Install [FFmpeg](https://ffmpeg.org/download.html) and ensure it's in PATH
2. Install [OBS Studio](https://obsproject.com/download) (for OBS Virtual Camera) **or** use Windows 11 built-in mode
3. Install [VB-CABLE](https://vb-audio.com/Cable/) (free virtual audio cable)
4. Run `pnpm install && pnpm tauri dev`
5. Go to **Setup** tab — verify all devices are detected
6. Go to **Dashboard** — select video/audio files and start streaming
7. In your meeting app, select "OBS Virtual Camera" as webcam and "CABLE Output" as microphone

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | Tauri v2 |
| Frontend | React + Vite + Tailwind CSS + shadcn/ui |
| Backend | Rust (100% media processing) |
| Video | ffmpeg CLI (OBS mode) / IMFVirtualCamera (built-in mode) |
| Audio | cpal + symphonia (decode & playback to selected output device) |
| COM DLL | `vcam-source` crate — IMFMediaSource for Windows Frame Server |
| Database | SQLite (sqlx) + Tantivy full-text search |
| Scripting | Rhai + JSON DSL |

## Development

### Prerequisites
- Node.js 18+ and pnpm 9+
- Rust 1.70+ targeting `x86_64-pc-windows-msvc`
- FFmpeg CLI in PATH

### Commands
```bash
pnpm install                  # Install frontend deps
pnpm tauri dev                # Full app dev with hot reload
pnpm tauri build              # Production build (NSIS installer)

# Quality checks
pnpm type-check               # TypeScript type checking
pnpm lint                     # ESLint
cargo fmt --all -- --check    # Rust formatting
cargo clippy --workspace      # Rust linting
cargo test --workspace        # Rust tests
npx vitest run                # Frontend tests

# Build vcam-source COM DLL (for built-in virtual camera)
cargo build -p vcam-source --release
# Register (admin): regsvr32 target\release\vcam_source.dll
```

### Project Structure

```
src/                          # React frontend
  components/
    Dashboard.tsx             # Main control panel (video/audio streaming)
    setup-panel.tsx           # Device detection & setup guide
    EnhancedMediaLibrary.tsx  # Media file browser
    Recording.tsx             # Recording controls
    Settings.tsx              # App settings
src-tauri/src/                # Rust backend
  virtual_device/
    webcam.rs                 # Dual-backend webcam (OBS + IMF)
    microphone.rs             # Audio playback to selected output device
    imf_webcam.rs             # IMFVirtualCamera frame pipeline
    shared_frame_buffer.rs    # Shared memory for COM DLL communication
    media_router.rs           # Coordinates video + audio streaming
  commands.rs                 # Tauri IPC command handlers
  commands_setup.rs           # Device detection & driver registration
  audio_decoder.rs            # Symphonia-based audio decoding
  devices/                    # Audio/video device enumeration
vcam-source/                  # COM DLL (IMFMediaSource for virtual camera)
  src/
    com_server.rs             # DLL exports + IClassFactory
    media_source.rs           # IMFMediaSource implementation
    media_stream.rs           # IMFMediaStream (frame delivery)
    frame_reader.rs           # Reads frames from shared memory
```

## License

MIT License — see [LICENSE](LICENSE) file for details
