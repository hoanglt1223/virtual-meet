# VirtualMeet - TODO & Implementation Status

## 📋 Project Overview

VirtualMeet is a Windows desktop application that simulates "virtual presence" by routing pre-recorded video and audio through virtual devices for online meeting applications.

### 🔥 Default F1-F12 Hotkey Mappings
- **Ctrl+F1**: Toggle Mute - Instantly mute/unmute microphone
- **Ctrl+F2**: Start Video - Begin video streaming
- **Ctrl+F3**: Stop Video - Stop video streaming
- **Ctrl+F4**: Screenshot - Take screenshot of current content
- **Ctrl+F5**: Start Recording - Begin video/audio recording
- **Ctrl+F6**: Stop Recording - Stop current recording session
- **Ctrl+F7**: Toggle Camera - Turn camera on/off
- **Ctrl+F8**: Start Audio - Begin audio streaming
- **Ctrl+F9**: Stop Audio - Stop audio streaming
- **Ctrl+F10**: Toggle Microphone - Turn microphone on/off
- **Ctrl+F11**: Open Settings - Open application settings
- **Ctrl+F12**: Quit Application - Exit the application
- **Shift+F11**: Volume Up - Increase microphone volume
- **Shift+F12**: Volume Down - Decrease microphone volume

## 🔧 Unimplemented Features (Code Review 2025-12-02)

### Backend (Rust)
- `start_audio_streaming()` / `stop_audio_streaming()` - requires AppState refactoring
- `search_media_library()` - empty results, use enhanced version
- `validate_media_file()` - always returns valid=true
- `set_current_video()` / `set_current_audio()` - validates but doesn't load into pipeline
- `register_hotkey()` / `unregister_hotkey()` - no OS-level registration
- `setup_global_hotkey_listener()` - no actual event listening
- `execute_action()` - Screenshot, ToggleCamera, ToggleMicrophone, VolumeUp/Down are stubs
- `check_key_combination_conflicts()` - always returns empty

### Frontend (React)
- `Dashboard.tsx` - Quick Action buttons have no onClick handlers
- `MediaLibrary.tsx` - uses empty mock data, Add Folder button has no handler
- `HotkeyManager.tsx` - Edit/Delete/Add Custom Hotkey buttons have no handlers

### Virtual Devices
- `webcam.rs` - DirectShow filter creation and frame delivery not implemented
- `microphone.rs` - WASAPI virtual device creation not implemented
