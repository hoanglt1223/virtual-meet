# Audio Pipeline Implementation

This document details the implementation of the audio pipeline foundation for VirtualMeet, providing core audio playback capabilities with MP3/PCM decoding, audio resampling, format conversion, volume control, and mute functionality.

## Overview

The audio pipeline follows the same architectural patterns established by the video pipeline, providing a modular, async-first implementation that integrates seamlessly with the existing Tauri application structure.

## Architecture

### Core Components

1. **Audio Module (`audio.rs`)** - Core data structures and utilities
2. **Audio Decoder (`audio_decoder.rs`)** - MP3/PCM decoding using Symphonia
3. **Audio Processor (`audio_processor.rs`)** - Real-time audio processing with volume control
4. **Virtual Microphone (`virtual/microphone.rs`)** - Integration with WASAPI and audio output

### Key Data Structures

#### AudioFrameData
```rust
pub struct AudioFrameData {
    pub data: Vec<u8>,           // Raw audio sample data
    pub channels: u32,          // Number of audio channels
    pub sample_rate: u32,       // Sample rate in Hz
    pub sample_format: AudioSampleFormat,
    pub timestamp: Duration,    // Frame timestamp
    pub frame_number: u64,      // Sequential frame counter
    pub duration: Duration,     // Frame duration
}
```

#### AudioBuffer
```rust
pub struct AudioBuffer {
    frames: Arc<StdMutex<VecDeque<AudioFrameData>>>,
    max_size: usize,
    total_frames: u64,
}
```

#### AudioProcessor
```rust
pub struct AudioProcessor {
    config: AudioConfig,
    is_muted: Arc<AtomicBool>,
    volume: Arc<AtomicF32>,
    processed_frames: u64,
    total_samples_processed: u64,
}
```

## Supported Formats

### Input Formats
- MP3 (.mp3)
- WAV (.wav)
- M4A/AAC (.m4a, .aac)
- OGG (.ogg)
- FLAC (.flac)

### Sample Formats
- F32 (32-bit float)
- I16 (16-bit signed integer)
- U16 (16-bit unsigned integer)
- I24 (24-bit signed integer)
- I32 (32-bit signed integer)

### Sample Rates
- 8 kHz to 192 kHz (configurable, default: 44.1 kHz)
- Automatic resampling to target sample rate

### Channel Configurations
- Mono (1 channel)
- Stereo (2 channels)
- Multi-channel (up to supported channels)

## Features

### 1. Audio Decoding

**Location:** `audio_decoder.rs`

**Technology:** Symphonia audio library

**Capabilities:**
- Multi-format audio decoding
- Metadata extraction
- Error recovery and handling
- Frame-based decoding pipeline

**Key Methods:**
```rust
pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()>
pub fn decode_all<P: AsRef<Path>>(&mut self, path: P, buffer_size: usize) -> Result<Vec<AudioFrameData>>
```

### 2. Audio Processing

**Location:** `audio_processor.rs`

**Features:**
- Volume control (0.0 to 1.0)
- Mute/unmute functionality
- Format conversion
- Sample rate resampling
- Real-time audio effects

**Key Methods:**
```rust
pub fn process_frame(&mut self, frame: AudioFrameData) -> Result<AudioFrameData>
pub fn set_volume(&self, volume: f32) -> Result<()>
pub fn set_muted(&self, muted: bool)
pub fn toggle_mute(&self) -> bool
```

### 3. Audio Visualization

**Location:** `audio_processor.rs` - `AudioVisualizer`

**Features:**
- Peak level detection
- RMS (Root Mean Square) calculation
- Multi-channel visualization
- Real-time level monitoring

**Key Methods:**
```rust
pub fn analyze_frame(&mut self, frame: &AudioFrameData) -> Result<AudioVisualizationData>
pub fn get_current_data(&self) -> AudioVisualizationData
```

### 4. Virtual Microphone Integration

**Location:** `virtual/microphone.rs`

**Features:**
- WASAPI integration (Windows)
- Multi-threaded audio processing
- Buffer management
- Device enumeration
- Streaming control

**Key Methods:**
```rust
pub async fn initialize(&self) -> Result<()>
pub async fn start_streaming(&mut self, audio_path: &str) -> Result<()>
pub async fn stop_streaming(&mut self) -> Result<()>
pub async fn set_volume(&self, volume: f32) -> Result<()>
pub async fn list_devices() -> Result<Vec<String>>
```

## Tauri Commands

### Core Audio Commands

1. **`init_microphone`** - Initialize virtual microphone
2. **`start_audio_streaming`** - Start audio playback from file
3. **`stop_audio_streaming`** - Stop audio playback
4. **`get_microphone_status`** - Get current microphone status
5. **`set_microphone_volume`** - Set volume level (0.0-1.0)
6. **`set_microphone_muted`** - Set mute state
7. **`toggle_microphone_mute`** - Toggle mute state
8. **`list_audio_devices`** - Enumerate audio devices
9. **`validate_audio_file`** - Validate audio file compatibility
10. **`get_supported_audio_formats`** - Get supported formats list

### Command Examples

```javascript
// Initialize microphone
await tauri.invoke('init_microphone');

// Start streaming audio file
await tauri.invoke('start_audio_streaming', {
  path: '/path/to/audio.mp3'
});

// Set volume to 50%
await tauri.invoke('set_microphone_volume', {
  volume: 0.5
});

// Toggle mute
await tauri.invoke('toggle_microphone_mute');

// Get microphone status
const status = await tauri.invoke('get_microphone_status');
console.log('Active:', status.is_active);
console.log('Volume:', status.volume);
console.log('Muted:', status.is_muted);
```

## Configuration

### AudioConfig

```rust
pub struct AudioConfig {
    pub output_sample_rate: u32,    // Default: 44100
    pub output_channels: u32,       // Default: 2 (stereo)
    pub output_format: AudioSampleFormat,  // Default: F32
    pub buffer_size: usize,         // Default: 8192
    pub volume: f32,               // Default: 1.0
    pub muted: bool,               // Default: false
}
```

## Threading Model

The audio pipeline uses a multi-threaded architecture:

1. **Main Thread:** Tauri command handling
2. **Decode Thread:** Audio file decoding and buffering
3. **Playback Thread:** Real-time audio output
4. **Processing Thread:** Audio effects and format conversion

### Thread Safety

- `Arc<Mutex<T>>` for shared state management
- `AtomicBool` and `AtomicF32` for performance-critical values
- Thread-safe audio buffer with circular buffer management

## Error Handling

### Error Types

1. **File I/O Errors:** Missing or corrupted audio files
2. **Format Errors:** Unsupported audio formats
3. **Decoding Errors:** Audio codec issues
4. **Processing Errors:** Memory or format conversion issues
5. **Device Errors:** Audio device enumeration or access issues

### Error Recovery

- Graceful degradation on decode errors
- Automatic retry mechanisms
- Fallback to safe defaults
- Comprehensive error logging

## Performance Considerations

### Memory Management

- Circular buffer with configurable size
- Frame-based processing to minimize memory usage
- Efficient format conversion algorithms
- Automatic buffer cleanup

### CPU Optimization

- SIMD-friendly audio processing
- Minimal audio buffer copying
- Lock-free atomic operations where possible
- Async/await for non-blocking operations

### Latency

- Configurable buffer sizes for latency vs. stability trade-offs
- Frame timestamps for synchronization
- Real-time processing capabilities
- Efficient audio pipeline

## Testing

### Unit Tests

**Location:** Each module contains comprehensive unit tests

**Coverage:**
- Audio buffer operations
- Format conversion
- Volume control
- Mute functionality
- Device enumeration
- Audio visualization

**Example:**
```rust
#[tokio::test]
async fn test_audio_buffer_operations() {
    let mut buffer = AudioBuffer::new(3);
    // Test buffer operations...
}
```

### Integration Tests

Integration tests cover:
- End-to-end audio processing pipeline
- File decoding workflows
- Multi-threaded operation
- Error handling scenarios

## Limitations and Future Improvements

### Current Limitations

1. **Virtual Audio Device:** Current implementation outputs through speakers
   - *Future:* True virtual microphone device implementation
   - *Requires:* Windows Audio Driver development

2. **Streaming Commands:** Some Tauri commands need state management improvements
   - *Future:* Refactor to use interior mutability patterns

3. **Resampling Quality:** Basic linear interpolation
   - *Future:* High-quality resampling algorithms

### Planned Enhancements

1. **Audio Effects:** Reverb, echo, filters
2. **Real-time Monitoring:** Audio level meters
3. **Multiple Audio Sources:** Audio mixing capabilities
4. **Advanced Configuration:** Per-user audio profiles
5. **Cross-platform Support:** macOS and Linux audio integration

## Dependencies

### Rust Dependencies

```toml
# Audio processing
rodio = "0.17"                    # Audio playback
symphonia = { version = "0.5", features = ["mp3", "wav", "isomp4"] }
cpal = "0.15"                     # Audio device abstraction

# Windows-specific
windows = { version = "0.52", features = [
    "Win32_Media_Audio",
    "Win32_Media_Audio_DirectSound",
    # ... other Windows audio features
] }
```

## Usage Examples

### Basic Audio Playback

```rust
use crate::virtual::microphone::VirtualMicrophone;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut microphone = VirtualMicrophone::new();

    // Initialize
    microphone.initialize().await?;

    // Start streaming
    microphone.start_streaming("audio.mp3").await?;

    // Control playback
    microphone.set_volume(0.7).await?;
    microphone.set_muted(false).await;

    // Get status
    let status = microphone.get_processing_stats().await;
    println!("Processed frames: {}", status.processed_frames);

    // Stop playback
    microphone.stop_streaming().await?;

    Ok(())
}
```

### Audio Processing Pipeline

```rust
use crate::audio_processor::{AudioProcessor, AudioConfig};

let config = AudioConfig::default();
let mut processor = AudioProcessor::new(config);

// Process audio frame
let processed_frame = processor.process_frame(audio_frame)?;

// Get visualization data
let viz_data = processor.get_visualization_data().await;
println!("Peak levels: {:?}", viz_data.peak_levels);
```

## Conclusion

The audio pipeline provides a solid foundation for audio playback and processing in VirtualMeet. It follows established patterns from the video pipeline while adding audio-specific capabilities like volume control, mute functionality, and real-time visualization.

The modular design allows for easy extension and customization, while the comprehensive test suite ensures reliability and maintainability. Future enhancements can build upon this foundation to provide more advanced audio features and true virtual device integration.