# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VirtualMeet is a Windows-only desktop application built with Tauri v2 that routes pre-recorded video and audio through virtual devices (OBS Virtual Camera, VB-CABLE, etc.) for use in online meeting apps. React frontend + Rust backend.

## Development Commands

```bash
pnpm install                  # Install frontend dependencies
pnpm tauri dev                # Full app dev with hot reload (frontend + Rust)
pnpm tauri build              # Production build (NSIS installer output)
pnpm dev                      # Frontend-only dev server (port 1420)
pnpm build                    # Build frontend only (tsc + vite)

# Quality checks
pnpm type-check               # TypeScript type checking (tsc --noEmit)
pnpm lint                     # ESLint
pnpm lint:fix                 # ESLint with auto-fix
cargo fmt --all -- --check    # Rust formatting check
cargo clippy --all-targets --all-features -- -D warnings  # Rust linting

# Testing
pnpm test                     # Run vitest (frontend)
pnpm test:watch               # Watch mode
pnpm test:coverage            # With coverage (v8)
cargo test --verbose          # Rust tests
```

## Architecture

### IPC Boundary

Frontend calls Rust via `@tauri-apps/api` invoke. All Tauri command handlers are registered in `src-tauri/src/main.rs` (the full handler list is there). Command modules are **flat files** in `src-tauri/src/`:

| Module | Purpose |
|---|---|
| `commands.rs` | Core device management, `AppState` init |
| `commands/virtual_devices.rs` | Virtual webcam/mic streaming & media routing |
| `commands_media.rs` | Media library CRUD and search |
| `commands_recording.rs` | Recording start/stop/config |
| `commands_hotkeys.rs` | Global hotkey registration |
| `commands_scripting.rs` | Rhai script management |
| `commands_json_dsl.rs` | JSON DSL script execution |
| `commands_settings.rs` | App settings get/set/export/import |

### Backend Modules (src-tauri/src/)

- **`virtual_device/`** — Webcam output, microphone output, media router (has `mod.rs` + submodules + `tests.rs`)
- **`recording/`** — Combined recorder, MP4 muxer, A/V sync, config (has `mod.rs` + submodules)
- **`devices/`** — Audio/video device enumeration and capability detection (has `mod.rs` + submodules)
- **`audio.rs` / `audio_decoder.rs` / `audio_processor.rs`** — Audio pipeline (rodio playback, cpal device access, symphonia decoding)
- **`media.rs` / `media_library.rs` / `media_scanner.rs` / `metadata_extractor.rs` / `thumbnail_generator.rs`** — SQLite media library with Tantivy full-text search
- **`scripting.rs`** — Rhai scripting engine
- **`json_dsl.rs` / `json_dsl_integration.rs`** — JSON DSL for automation
- **`hotkeys.rs`** — Global hotkey system via `global-hotkey` crate
- **`error.rs`** — Custom error types with anyhow

### Frontend (src/)

- **`App.tsx`** — Tab-based layout: Dashboard, Media Library, Recording, Settings
- **`components/`** — One component per tab (`Dashboard.tsx`, `EnhancedMediaLibrary.tsx`, `Recording.tsx`, `Settings.tsx`, `HotkeyManager.tsx`)
- **`components/ui/`** — shadcn/ui primitives (Radix UI + Tailwind)
- **`types/index.ts`** — All TypeScript types for IPC (must match Rust structs)
- Path alias: `@/` maps to `./src/` (configured in both `tsconfig.json` and `vite.config.ts`)

### State Management

- **Rust**: Multiple Tauri managed states initialized in `main.rs` setup: `AppState`, `VirtualDeviceState`, hotkey state, scripting state, JSON DSL state, recording state
- **Frontend**: React `useState` only — no external state management library

## Key Dependencies

| Layer | Key Crates/Packages |
|---|---|
| Video | `ffmpeg-next`, `windows-capture`, `image` |
| Audio | `rodio`, `cpal`, `symphonia` |
| Database | `sqlx` (SQLite, async with Tokio) |
| Search | `tantivy` |
| Scripting | `rhai` |
| Hotkeys | `global-hotkey` |
| Windows APIs | `windows` crate (DirectShow, WASAPI, Media Foundation) |
| Frontend UI | `@radix-ui/*`, `tailwindcss`, `lucide-react` |

## Build Requirements

- **FFmpeg**: Required for Rust `ffmpeg-next` crate. CI installs via Chocolatey and sets `FFMPEG_DIR`, `FFMPEG_STATIC=1`, `PKG_CONFIG_PATH` env vars.
- **Node.js 18+** and **pnpm 9+**
- **Rust 1.70+** targeting `x86_64-pc-windows-msvc`
- **Tauri CLI v2**: `cargo install tauri-cli --version "^2.0.0"`

## Testing

- Frontend tests: Vitest + Testing Library + jsdom. Test setup in `src/test/setup.ts` mocks `__TAURI__` global (invoke/listen/emit).
- Rust tests: `cargo test`. Virtual device module has dedicated `tests.rs`.
- CI runs both in parallel (`quality-check` job in `.github/workflows/build-deploy.yml`).

## Development Notes

- Windows-only: uses WASAPI, DirectShow, Media Foundation APIs throughout the Rust backend
- All media processing is in Rust — frontend does zero media work
- Virtual devices require third-party drivers installed (OBS Virtual Camera, VB-CABLE, VoiceMeeter)
- Vite dev server runs on port 1420 (strict port, required by Tauri)
- Tauri plugins enabled: `shell`, `dialog`, `fs`
