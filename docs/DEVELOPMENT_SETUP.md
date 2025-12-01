# Development Setup Guide

## Prerequisites Installation

### 1. Install Node.js and pnpm
```bash
# Install Node.js 18+ from https://nodejs.org/
# Then install pnpm globally
npm install -g pnpm
```

### 2. Install Rust
```bash
# Install Rust from https://rustup.rs/
# After installation, verify:
cargo --version
rustc --version
```

### 3. Install Windows Target for Rust
```bash
rustup target add x86_64-pc-windows-msvc
```

### 4. Install Tauri CLI
```bash
cargo install tauri-cli --version "^2.0.0"
```

### 5. Install Virtual Drivers
- **Virtual Webcam**: OBS Studio (with Virtual Camera), ManyCam, or similar
- **Virtual Audio**: VB-CABLE, VoiceMeeter, or similar

## Project Setup

### 1. Install Dependencies
```bash
pnpm install
```

### 2. Verify Tauri Setup
```bash
pnpm tauri --version
```

### 3. Start Development Server
```bash
pnpm tauri dev
```

### 4. Build for Production
```bash
pnpm tauri build
```

## Environment Verification Checklist

- [ ] Node.js 18+ installed
- [ ] pnpm installed and working
- [ ] Rust installed with Windows target
- [ ] Tauri CLI v2 installed
- [ ] Virtual webcam driver installed
- [ ] Virtual audio driver installed
- [ ] Project dependencies installed
- [ ] Development server starts successfully

## Common Issues

### Rust Not Found
```bash
# Make sure Rust is installed and in PATH
echo $PATH
cargo --version
```

### Tauri Build Fails
```bash
# Clean and rebuild
pnpm tauri clean
pnpm tauri dev
```

### Virtual Device Issues
- Restart the virtual device drivers
- Check Windows Sound/Camera settings
- Ensure virtual devices are enabled in meeting apps