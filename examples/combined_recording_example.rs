//! Combined Recording Pipeline Example
//!
//! This example demonstrates how to use the combined recording pipeline
//! to capture audio and video to MP4 files with configurable settings.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;

// Import the recording modules (this would be in your actual crate)
use virtualmeet::recording::{
    CombinedRecorder, RecordingConfig, VideoResolution, VideoQualityPreset,
    AudioQualityPreset, FrameRate, VideoCodec, AudioCodec,
    recorder_utils::{create_video_frame_rgb, create_audio_frame},
};
use virtualmeet::audio::AudioSampleFormat;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Example 1: Simple 1080p recording with default settings
    simple_1080p_recording()?;

    // Example 2: Custom high-quality recording
    custom_high_quality_recording()?;

    // Example 3: Fast recording for performance
    fast_performance_recording()?;

    // Example 4: Real-time recording with frame submission
    real_time_recording_example()?;

    Ok(())
}

/// Example 1: Simple 1080p recording with default settings
fn simple_1080p_recording() -> Result<()> {
    println!("=== Simple 1080p Recording Example ===");

    // Create a 1080p recording configuration
    let config = RecordingConfig::hd_1080p();

    // Create the recorder
    let mut recorder = CombinedRecorder::new(config)?;
    let output_path = "example_1080p.mp4";

    println!("Starting recording to: {}", output_path);

    // Start recording
    let session_id = recorder.start_recording(output_path)?;
    println!("Recording started with session ID: {}", session_id);

    // Simulate recording for 5 seconds
    std::thread::sleep(Duration::from_secs(2));

    // Stop recording
    recorder.stop_recording()?;
    println!("Recording stopped");

    // Get statistics
    let stats = recorder.get_stats()?;
    println!("Recording statistics: {:?}", stats);

    Ok(())
}

/// Example 2: Custom high-quality recording
fn custom_high_quality_recording() -> Result<()> {
    println!("\n=== Custom High-Quality Recording Example ===");

    // Create a custom configuration for high quality
    let mut config = RecordingConfig::default();
    config.video.resolution = VideoResolution::HD1080p;
    config.video.frame_rate = FrameRate::FPS60;
    config.video.quality_preset = VideoQualityPreset::High;
    config.video.codec = VideoCodec::H265;
    config.audio.quality_preset = AudioQualityPreset::High;
    config.audio.sample_rate = 48000;
    config.audio.channels = 2;

    // Validate the configuration
    config.validate()?;

    // Create the recorder
    let mut recorder = CombinedRecorder::new(config)?;
    let output_path = "example_high_quality.mp4";

    println!("Starting high-quality recording to: {}", output_path);

    // Start recording
    let session_id = recorder.start_recording(output_path)?;
    println!("High-quality recording started with session ID: {}", session_id);

    // Simulate recording
    std::thread::sleep(Duration::from_secs(1));

    // Stop recording
    recorder.stop_recording()?;
    println!("High-quality recording stopped");

    Ok(())
}

/// Example 3: Fast recording for performance
fn fast_performance_recording() -> Result<()> {
    println!("\n=== Fast Performance Recording Example ===");

    // Create a fast recording configuration
    let config = RecordingConfig::fast_recording();

    // Create the recorder
    let mut recorder = CombinedRecorder::new(config)?;
    let output_path = "example_fast.mp4";

    println!("Starting fast recording to: {}", output_path);

    // Start recording
    let session_id = recorder.start_recording(output_path)?;
    println!("Fast recording started with session ID: {}", session_id);

    // Simulate recording
    std::thread::sleep(Duration::from_secs(1));

    // Stop recording
    recorder.stop_recording()?;
    println!("Fast recording stopped");

    Ok(())
}

/// Example 4: Real-time recording with frame submission
fn real_time_recording_example() -> Result<()> {
    println!("\n=== Real-Time Recording Example ===");

    // Create a configuration
    let config = RecordingConfig::hd_720p();
    let mut recorder = CombinedRecorder::new(config)?;
    let output_path = "example_realtime.mp4";

    println!("Starting real-time recording to: {}", output_path);

    // Start recording
    let session_id = recorder.start_recording(output_path)?;
    println!("Real-time recording started with session ID: {}", session_id);

    // Simulate submitting frames for 3 seconds
    let frame_duration = Duration::from_millis(33); // ~30fps
    let frame_count = 90; // 3 seconds at 30fps

    for i in 0..frame_count {
        // Create a video frame (simulated)
        let video_data = vec![255u8; 1280 * 720 * 3]; // 720p RGB frame
        let video_frame = create_video_frame_rgb(
            video_data,
            1280,
            720,
            frame_duration * i,
            i,
        );

        // Submit video frame
        recorder.submit_video_frame(video_frame)?;

        // Create an audio frame (simulated)
        let audio_data = vec![0u8; 1024 * 2 * 4]; // 1024 samples, 2 channels, f32
        let audio_frame = create_audio_frame(
            audio_data,
            2,
            44100,
            AudioSampleFormat::F32,
            frame_duration * i,
            i,
        );

        // Submit audio frame
        recorder.submit_audio_frame(audio_frame)?;

        // Print progress
        if i % 30 == 0 {
            println!("Submitted {} frames...", i);
        }

        // Simulate real-time frame rate
        std::thread::sleep(frame_duration);
    }

    // Stop recording
    recorder.stop_recording()?;
    println!("Real-time recording stopped");

    // Get final statistics
    let stats = recorder.get_stats()?;
    println!("Final statistics:");
    println!("  Video frames: {}", stats.video_frames_recorded);
    println!("  Audio frames: {}", stats.audio_frames_recorded);
    println!("  File size: {} bytes", stats.disk_usage);

    Ok(())
}

/// Example 5: Configuration exploration
fn explore_configurations() -> Result<()> {
    println!("\n=== Configuration Exploration Example ===");

    // Test different resolutions
    let resolutions = vec![
        VideoResolution::VGA,
        VideoResolution::HD720p,
        VideoResolution::HD1080p,
    ];

    for resolution in resolutions {
        let (width, height) = resolution.dimensions();
        let pixel_count = resolution.pixel_count();
        println!("Resolution: {} ({}x{}) - {} pixels",
                resolution.name(), width, height, pixel_count);
    }

    // Test quality presets
    let quality_presets = vec![
        VideoQualityPreset::Fast,
        VideoQualityPreset::Balanced,
        VideoQualityPreset::High,
        VideoQualityPreset::Ultra,
    ];

    for preset in quality_presets {
        let crf = preset.crf_value();
        let bitrate_1080p = preset.recommended_bitrate_1080p();
        println!("Quality: {} - CRF: {}, 1080p bitrate: {} Mbps",
                preset.name(), crf, bitrate_1080p / 1_000_000);
    }

    // Test audio quality presets
    let audio_presets = vec![
        AudioQualityPreset::Low,
        AudioQualityPreset::Voice,
        AudioQualityPreset::Standard,
        AudioQualityPreset::High,
        AudioQualityPreset::Lossless,
    ];

    for preset in audio_presets {
        let bitrate = preset.recommended_bitrate();
        let sample_rate = preset.recommended_sample_rate();
        println!("Audio: {} - Bitrate: {} kbps, Sample rate: {} Hz",
                preset.name(), bitrate / 1000, sample_rate);
    }

    Ok(())
}

/// Example 6: Error handling and recovery
fn error_handling_example() -> Result<()> {
    println!("\n=== Error Handling Example ===");

    // Test invalid configuration
    let mut invalid_config = RecordingConfig::default();
    invalid_config.video.frame_rate = FrameRate::Custom(0.0); // Invalid frame rate

    match invalid_config.validate() {
        Ok(_) => println!("Configuration is valid"),
        Err(e) => println!("Configuration error: {}", e),
    }

    // Test file creation issues
    let config = RecordingConfig::default();
    let mut recorder = CombinedRecorder::new(config)?;

    // Try to start recording with invalid path
    let invalid_path = "/invalid/path/that/does/not/exist/test.mp4";
    match recorder.start_recording(invalid_path) {
        Ok(_) => println!("Recording started successfully"),
        Err(e) => println!("Recording failed: {}", e),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_presets() {
        // Test that presets create valid configurations
        let hd_720p = RecordingConfig::hd_720p();
        assert!(hd_720p.validate().is_ok());

        let hd_1080p = RecordingConfig::hd_1080p();
        assert!(hd_1080p.validate().is_ok());

        let fast = RecordingConfig::fast_recording();
        assert!(fast.validate().is_ok());
    }

    #[test]
    fn test_video_frame_creation() {
        let video_data = vec![255u8; 640 * 480 * 3];
        let frame = create_video_frame_rgb(
            video_data,
            640,
            480,
            Duration::from_millis(0),
            1,
        );

        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.frame_number, 1);
    }

    #[test]
    fn test_audio_frame_creation() {
        let audio_data = vec![0u8; 1024 * 2 * 4];
        let frame = create_audio_frame(
            audio_data,
            2,
            44100,
            AudioSampleFormat::F32,
            Duration::from_millis(0),
            1,
        );

        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_rate, 44100);
        assert_eq!(frame.sample_format, AudioSampleFormat::F32);
    }

    #[test]
    fn test_configuration_validation() {
        let mut config = RecordingConfig::default();

        // Valid configuration
        assert!(config.validate().is_ok());

        // Invalid frame rate
        config.video.frame_rate = FrameRate::Custom(0.0);
        assert!(config.validate().is_err());

        // Invalid sample rate
        config.video.frame_rate = FrameRate::FPS30;
        config.audio.sample_rate = 0;
        assert!(config.validate().is_err());
    }
}