# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VirtualMeet is a Windows desktop application built with Tauri v2 that simulates a "virtual presence" by routing pre-recorded video and audio through virtual devices for use in online meeting applications. The app combines a React frontend with a comprehensive Rust backend for media processing, device management, and automation.

## Development Commands

```bash
# Install dependencies
pnpm install

# Start development server with hot reload
pnpm tauri dev

# Build for production
pnpm tauri build

# Frontend-only development (useful for UI testing)
pnpm dev

# Build frontend only
pnpm build

# TypeScript compilation
tsc

# Rust-specific commands (from project root)
cargo build                    # Build Rust backend
cargo run                      # Run Rust backend only
cargo test                     # Run Rust tests
cargo clippy                   # Lint Rust code
```

## Architecture

### Core Structure

- **Frontend (`src/`)**: React + Vite + Tailwind CSS + shadcn/ui
- **Backend (`src-tauri/`)**: Pure Rust with Tauri v2 for desktop app shell
- **Architecture Pattern**: Frontend communicates with Rust backend via Tauri's IPC commands

### Backend Architecture

The Rust backend is organized into key modules:

- **`commands/`**: Tauri command handlers split by functionality
  - `commands.rs`: Core device management (webcam/microphone)
  - `commands_media.rs`: Media library operations
  - `commands_recording.rs`: Recording functionality
  - `commands_hotkeys.rs`: Global hotkey management
  - `commands_scripting.rs`: Script execution and management
  - `commands_settings.rs`: Configuration management

- **`virtual/`**: Virtual device implementation
  - `webcam.rs`: Virtual webcam video output
  - `microphone.rs`: Virtual microphone audio output
  - `media_router.rs`: Media routing between inputs and outputs

- **`media/`**: Media processing and management
  - `media_library.rs`: SQLite-based media indexing
  - `media_scanner.rs`: File system scanning
  - `metadata_extractor.rs`: Media file analysis
  - `thumbnail_generator.rs`: Video thumbnail creation

- **`recording/`**: Combined video/audio recording system
- **`devices/`**: Device enumeration and capability detection
- **`audio/`**: Audio processing pipeline with ffmpeg and rodio
- **`scripting/`**: Rhai-based scripting engine
- **`hotkeys/`**: Global hotkey registration and handling

### Frontend Structure

- **`App.tsx`**: Main application with tab-based navigation
- **`components/`**: Feature-specific React components
  - `Dashboard.tsx`: Main control panel with video preview
  - `MediaLibrary.tsx`: Media browser and management
  - `Recording.tsx`: Recording controls and status
  - `Settings.tsx`: Configuration interface
- **`components/ui/`**: shadcn/ui component library
- **`types/`**: TypeScript type definitions for IPC commands

### Key Technical Details

- **Database**: SQLite for media library metadata and settings
- **Video Processing**: ffmpeg-next for decoding, windows-capture for integration
- **Audio Processing**: rodio for playback, cpal for device access, symphonia for decoding
- **Search**: Tantivy for full-text media search
- **Async Architecture**: Tokio runtime throughout the Rust backend
- **Error Handling**: anyhow for error propagation with custom error types

### Device Integration Strategy

The application works with existing virtual device drivers:
- Virtual webcam: OBS Virtual Camera, ManyCam, etc.
- Virtual audio: VB-CABLE, VoiceMeeter, etc.
- Physical device enumeration via Windows APIs (DirectShow, WASAPI)

### State Management

- **Rust Backend**: Shared state managed through Tauri's app state management
- **Frontend**: React useState for local component state
- **Communication**: Tauri commands for all frontend-backend communication

### Key Command Categories

1. **Device Commands**: Initialize, start/stop streaming, get status
2. **Media Commands**: Load, search, validate media files
3. **Recording Commands**: Start/stop recording, configure presets
4. **Hotkey Commands**: Register/unregister global hotkeys
5. **Scripting Commands**: Execute Rhai scripts with JSON/DSL support
6. **Settings Commands**: Manage configuration and device preferences

## Development Notes

- The project targets Windows specifically (WASAPI, DirectShow, Media Foundation APIs)
- All media processing happens in Rust for performance
- Frontend is purely for UI/UX, no direct media processing
- Global hotkeys work system-wide, even when app is not focused
- Scripting engine allows automation of media switching and device control