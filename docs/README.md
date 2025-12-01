# VirtualMeet

An open-source Windows desktop app that simulates a "virtual presence" by sending pre-recorded video & audio into online meeting apps as a virtual webcam and virtual microphone.

## Features

- **Virtual Webcam**: Play MP4 videos as webcam output with looping and fast switching
- **Virtual Microphone**: Play MP3/audio files as microphone input with volume control
- **Recording**: Record combined video + audio to MP4 files
- **Media Library**: Quick access to media files with thumbnails
- **Hotkeys & Automation**: Global hotkeys and simple scripting engine
- **Device Integration**: Works with existing virtual webcam and audio drivers

## Tech Stack

- **Shell**: Tauri v2 desktop app
- **Frontend**: React + Vite + Tailwind CSS + shadcn/ui
- **Core**: 100% Rust for media processing, playback, and automation
- **Platform**: Windows (with WASAPI, DirectShow, Media Foundation)

## Development Setup

### Prerequisites
- Node.js 18+ and pnpm
- Rust 1.70+ with target `x86_64-pc-windows-msvc`
- Tauri CLI: `cargo install tauri-cli --version "^2.0.0"`

### Installation
```bash
# Install dependencies
pnpm install

# Start development server
pnpm tauri dev

# Build for production
pnpm tauri build
```

## Quick Start

1. Install a virtual webcam driver (e.g., OBS Virtual Camera, ManyCam)
2. Install a virtual audio driver (e.g., VB-CABLE, VoiceMeeter)
3. Run `pnpm install` and `pnpm tauri dev`
4. Configure your virtual devices in settings
5. Add media files to your library
6. Select video/audio and start using in meeting apps

## Project Status

🚧 **In Development** - Core infrastructure and architecture planning phase

## License

MIT License - see [LICENSE](LICENSE) file for details