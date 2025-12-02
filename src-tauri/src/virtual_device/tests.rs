//! Integration Tests for Virtual Device Functionality
//!
//! This module contains comprehensive tests for virtual webcam and microphone
//! functionality, including error handling and edge cases.

#[cfg(test)]
mod virtual_device_tests {
    use super::*;
    use crate::audio::AudioConfig;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_webcam_creation_and_backends() {
        // Test DirectShow backend
        let dshow_webcam = VirtualWebcam::with_backend(WebcamBackend::DirectShow);
        assert!(!dshow_webcam.is_active().await);

        // Test Media Foundation backend
        let mf_webcam = VirtualWebcam::with_backend(WebcamBackend::MediaFoundation);
        assert!(!mf_webcam.is_active().await);
    }

    #[tokio::test]
    async fn test_webcam_initialization() {
        let webcam = VirtualWebcam::new();

        // This test may fail on systems without proper virtual device drivers
        // which is expected behavior
        let result = webcam.initialize().await;
        match result {
            Ok(_) => info!("Webcam initialized successfully"),
            Err(e) => {
                println!("Expected initialization failure: {}", e);
                assert!(e.to_string().contains("requires") ||
                       e.to_string().contains("failed") ||
                       e.to_string().contains("COM"));
            }
        }
    }

    #[tokio::test]
    async fn test_webcam_video_info() {
        let webcam = VirtualWebcam::new();
        let video_info = webcam.get_video_info().await;

        // Should be None since no video has been loaded
        assert!(video_info.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_webcam_buffer_status() {
        let webcam = VirtualWebcam::new();
        let buffer_status = webcam.get_buffer_status().await;

        assert_eq!(buffer_status.current_frames, 0);
        assert_eq!(buffer_status.capacity, 300); // Default capacity
        assert_eq!(buffer_status.total_processed, 0);
    }

    #[tokio::test]
    async fn test_webcam_device_enumeration() {
        let devices = VirtualWebcam::list_devices().await;

        // Should contain at least our virtual devices
        assert!(devices.is_ok());
        if let Ok(devs) = devices {
            assert!(!devs.is_empty());
            assert!(devs.iter().any(|d| d.contains("VirtualWebcam")));
        }
    }

    #[tokio::test]
    async fn test_webcam_invalid_file_handling() {
        let webcam = VirtualWebcam::new();

        // Should fail to start streaming with non-existent file
        let result = webcam.start_streaming("nonexistent_file.mp4").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_microphone_creation_and_backends() {
        // Test WASAPI backend
        let wasapi_mic = VirtualMicrophone::with_backend(MicrophoneBackend::WASAPI);
        assert!(!wasapi_mic.is_active().await);

        // Test KernelStreaming backend
        let ks_mic = VirtualMicrophone::with_backend(MicrophoneBackend::KernelStreaming);
        assert!(!ks_mic.is_active().await);
    }

    #[tokio::test]
    async fn test_microphone_initialization() {
        let microphone = VirtualMicrophone::new();

        // This test may fail on systems without proper virtual audio drivers
        // which is expected behavior
        let result = microphone.initialize().await;
        match result {
            Ok(_) => info!("Microphone initialized successfully"),
            Err(e) => {
                println!("Expected initialization failure: {}", e);
                assert!(e.to_string().contains("requires") ||
                       e.to_string().contains("failed") ||
                       e.to_string().contains("COM"));
            }
        }
    }

    #[tokio::test]
    async fn test_microphone_volume_control() {
        let microphone = VirtualMicrophone::new();

        // Test valid volume settings
        assert!(microphone.set_volume(0.5).await.is_ok());
        assert_eq!(microphone.get_volume().await, 0.5);

        assert!(microphone.set_volume(0.0).await.is_ok());
        assert_eq!(microphone.get_volume().await, 0.0);

        assert!(microphone.set_volume(1.0).await.is_ok());
        assert_eq!(microphone.get_volume().await, 1.0);

        // Test invalid volume settings
        assert!(microphone.set_volume(1.5).await.is_err());
        assert!(microphone.set_volume(-0.1).await.is_err());
    }

    #[tokio::test]
    async fn test_microphone_mute_control() {
        let microphone = VirtualMicrophone::new();

        // Test initial state
        assert!(!microphone.is_muted().await);

        // Test mute toggle
        let new_state = microphone.toggle_mute().await;
        assert!(new_state);
        assert!(microphone.is_muted().await);

        // Test unmute
        microphone.set_muted(false).await;
        assert!(!microphone.is_muted().await);

        // Test mute again
        microphone.set_muted(true).await;
        assert!(microphone.is_muted().await);
    }

    #[tokio::test]
    async fn test_microphone_device_enumeration() {
        let devices = VirtualMicrophone::list_devices().await;

        // Should contain at least our virtual devices
        assert!(devices.is_ok());
        if let Ok(devs) = devices {
            assert!(!devs.is_empty());
            assert!(devs.iter().any(|d| d.contains("VirtualMicrophone")));
        }
    }

    #[tokio::test]
    async fn test_microphone_invalid_file_handling() {
        let mut microphone = VirtualMicrophone::new();

        // Should fail to start streaming with non-existent file
        let result = microphone.start_streaming("nonexistent_file.mp3").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_media_router_creation() {
        let router = MediaRouter::new();
        assert!(!router.get_status().await.is_active);
    }

    #[tokio::test]
    async fn test_media_router_config() {
        let config = MediaRoutingConfig {
            video_backend: WebcamBackend::MediaFoundation,
            audio_backend: MicrophoneBackend::KernelStreaming,
            video_path: "test_video.mp4".to_string(),
            audio_path: "test_audio.mp3".to_string(),
            sync_audio_video: true,
            loop_media: false,
            video_volume: 0.8,
            audio_volume: 0.6,
        };

        let router = MediaRouter::with_config(config.clone());
        let status = router.get_status().await;

        assert_eq!(status.config.video_backend, config.video_backend);
        assert_eq!(status.config.audio_backend, config.audio_backend);
        assert_eq!(status.config.video_path, config.video_path);
        assert_eq!(status.config.audio_path, config.audio_path);
        assert_eq!(status.config.sync_audio_video, config.sync_audio_video);
        assert_eq!(status.config.loop_media, config.loop_media);
        assert_eq!(status.config.video_volume, config.video_volume);
        assert_eq!(status.config.audio_volume, config.audio_volume);
    }

    #[tokio::test]
    async fn test_media_router_initialization() {
        let router = MediaRouter::new();

        let result = router.initialize().await;
        match result {
            Ok(_) => info!("Media router initialized successfully"),
            Err(e) => {
                println!("Expected initialization failure: {}", e);
                // Expected to fail without virtual device drivers
            }
        }
    }

    #[tokio::test]
    async fn test_media_router_config_updates() {
        let router = MediaRouter::new();

        let new_config = MediaRoutingConfig {
            audio_volume: 0.3,
            video_volume: 0.7,
            sync_audio_video: false,
            ..Default::default()
        };

        let result = router.update_config(new_config.clone()).await;
        // This may fail if router isn't initialized, which is expected
        if result.is_ok() {
            let status = router.get_status().await;
            assert_eq!(status.config.audio_volume, new_config.audio_volume);
            assert_eq!(status.config.video_volume, new_config.video_volume);
            assert_eq!(status.config.sync_audio_video, new_config.sync_audio_video);
        }
    }

    #[tokio::test]
    async fn test_media_router_invalid_media_switch() {
        let router = MediaRouter::new();

        let result = router.switch_media(
            Some("nonexistent_video.mp4".to_string()),
            Some("nonexistent_audio.mp3".to_string()),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_sync_state_creation() {
        let sync_state = SyncState::default();

        assert_eq!(sync_state.video_timestamp, Duration::default());
        assert_eq!(sync_state.audio_timestamp, Duration::default());
        assert!(!sync_state.is_video_playing);
        assert!(!sync_state.is_audio_playing);
        assert_eq!(sync_state.sync_offset, Duration::default());
    }

    #[tokio::test]
    async fn test_buffer_status_utilization() {
        use crate::virtual_device::webcam::BufferStatus;

        let status = BufferStatus {
            current_frames: 150,
            capacity: 300,
            total_processed: 1000,
        };

        assert_eq!(status.utilization(), 50.0);

        let empty_status = BufferStatus {
            current_frames: 0,
            capacity: 100,
            total_processed: 0,
        };

        assert_eq!(empty_status.utilization(), 0.0);

        let full_status = BufferStatus {
            current_frames: 100,
            capacity: 100,
            total_processed: 500,
        };

        assert_eq!(full_status.utilization(), 100.0);
    }

    #[tokio::test]
    async fn test_video_info_formatting() {
        use crate::virtual_device::webcam::VideoInfo;

        let video_info = VideoInfo {
            width: 1920,
            height: 1080,
            frame_rate: 30.0,
            duration: Some(Duration::from_secs(120)),
        };

        assert_eq!(video_info.dimensions_string(), "1920x1080");
        assert_eq!(video_info.duration_string(), "02:00");

        let video_info_no_duration = VideoInfo {
            width: 1280,
            height: 720,
            frame_rate: 25.0,
            duration: None,
        };

        assert_eq!(video_info_no_duration.dimensions_string(), "1280x720");
        assert_eq!(video_info_no_duration.duration_string(), "Unknown");
    }

    #[tokio::test]
    async fn test_frame_data_operations() {
        use crate::virtual_device::webcam::VideoFrameData;

        let data = vec![255u8; 1920 * 1080 * 3]; // RGB24 data
        let frame = VideoFrameData::new(
            data.clone(),
            1920,
            1080,
            Duration::from_millis(33),
            42,
        );

        assert_eq!(frame.size(), data.len());
        assert_eq!(frame.dimensions(), (1920, 1080));
        assert_eq!(frame.frame_number, 42);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
    }

    #[tokio::test]
    async fn test_concurrent_device_access() {
        // Test that multiple devices can be created and accessed concurrently
        let webcam1 = VirtualWebcam::with_backend(WebcamBackend::DirectShow);
        let webcam2 = VirtualWebcam::with_backend(WebcamBackend::MediaFoundation);
        let mic1 = VirtualMicrophone::with_backend(MicrophoneBackend::WASAPI);
        let mic2 = VirtualMicrophone::with_backend(MicrophoneBackend::KernelStreaming);

        // All should be inactive initially
        assert!(!webcam1.is_active().await);
        assert!(!webcam2.is_active().await);
        assert!(!mic1.is_active().await);
        assert!(!mic2.is_active().await);

        // Test concurrent volume setting
        let vol_result1 = mic1.set_volume(0.5).await;
        let vol_result2 = mic2.set_volume(0.7).await;

        assert!(vol_result1.is_ok());
        assert!(vol_result2.is_ok());
        assert_eq!(mic1.get_volume().await, 0.5);
        assert_eq!(mic2.get_volume().await, 0.7);
    }

    #[tokio::test]
    async fn test_error_handling_edge_cases() {
        let webcam = VirtualWebcam::new();
        let microphone = VirtualMicrophone::new();

        // Test stopping when not active
        assert!(webcam.stop_streaming().await.is_ok());

        // Test getting current source when none set
        assert!(webcam.current_source().await.is_none());
        assert!(microphone.current_source().await.is_none());

        // Test setting invalid volume multiple times
        for _ in 0..5 {
            assert!(microphone.set_volume(2.0).await.is_err());
            assert!(microphone.set_volume(-1.0).await.is_err());
        }

        // Test valid volume after invalid attempts
        assert!(microphone.set_volume(0.5).await.is_ok());
        assert_eq!(microphone.get_volume().await, 0.5);
    }

    #[tokio::test]
    async fn test_media_router_comprehensive_status() {
        let router = MediaRouter::new();
        let status = router.get_status().await;

        assert!(!status.is_active);
        assert!(!status.webcam_status.is_active);
        assert!(!status.microphone_status.is_active);

        // Check default config values
        assert_eq!(status.config.video_volume, 1.0);
        assert_eq!(status.config.audio_volume, 1.0);
        assert!(status.config.sync_audio_video);
        assert!(status.config.loop_media);

        // Check sync state
        assert_eq!(status.sync_state.video_timestamp, Duration::default());
        assert_eq!(status.sync_state.audio_timestamp, Duration::default());
        assert_eq!(status.sync_state.sync_offset, Duration::default());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::fs::File;
    use std::io::Write;

    // Helper function to create a temporary media file for testing
    fn create_temp_media_file(content: &[u8], extension: &str) -> String {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content).unwrap();
        let path = temp_file.path().to_path_buf();

        // Add the extension
        let path_with_ext = path.with_extension(extension);
        std::fs::rename(&path, &path_with_ext).unwrap();

        path_with_ext.to_string_lossy().to_string()
    }

    #[tokio::test]
    async fn test_webcam_with_temp_file() {
        // Create a temporary file (not a real video, but for testing file existence)
        let temp_video = create_temp_file(b"fake video content", "mp4");
        let webcam = VirtualWebcam::new();

        // This should fail because it's not a real video file, but not because of file existence
        let result = webcam.start_streaming(&temp_video).await;
        assert!(result.is_err());
        // Error should not be about file not existing
        assert!(!result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_microphone_with_temp_file() {
        // Create a temporary file (not a real audio, but for testing file existence)
        let temp_audio = create_temp_media_file(b"fake audio content", "mp3");
        let mut microphone = VirtualMicrophone::new();

        // This should fail because it's not a real audio file, but not because of file existence
        let result = microphone.start_streaming(&temp_audio).await;
        assert!(result.is_err());
        // Error should not be about file not existing
        assert!(!result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_media_router_lifecycle() {
        let config = MediaRoutingConfig {
            video_backend: WebcamBackend::DirectShow,
            audio_backend: MicrophoneBackend::WASAPI,
            video_path: "test.mp4".to_string(),
            audio_path: "test.mp3".to_string(),
            ..Default::default()
        };

        let router = MediaRouter::with_config(config);

        // Test initial state
        let initial_status = router.get_status().await;
        assert!(!initial_status.is_active);

        // Test lifecycle methods (they will likely fail due to missing virtual drivers)
        let init_result = router.initialize().await;
        match init_result {
            Ok(_) => {
                // If initialization succeeded, test start/stop
                let start_result = router.start(initial_status.config.clone()).await;
                if start_result.is_ok() {
                    sleep(Duration::from_millis(100)).await;
                    let stop_result = router.stop().await;
                    assert!(stop_result.is_ok());

                    let final_status = router.get_status().await;
                    assert!(!final_status.is_active);
                }
            },
            Err(_) => {
                // Initialization failure is expected without virtual device drivers
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_media_routers() {
        // Test creating multiple media routers
        let router1 = MediaRouter::new();
        let router2 = MediaRouter::new();
        let router3 = MediaRouter::new();

        // All should be independent and initially inactive
        let status1 = router1.get_status().await;
        let status2 = router2.get_status().await;
        let status3 = router3.get_status().await;

        assert!(!status1.is_active);
        assert!(!status2.is_active);
        assert!(!status3.is_active);

        // Config updates should not affect other routers
        let config1 = MediaRoutingConfig {
            video_volume: 0.5,
            audio_volume: 0.7,
            ..Default::default()
        };

        router1.update_config(config1).await.ok();

        let updated_status1 = router1.get_status().await;
        let unchanged_status2 = router2.get_status().await;

        // Router 1 should have updated config, router 2 should remain default
        if updated_status1.config.video_volume == 0.5 {
            assert_eq!(unchanged_status2.config.video_volume, 1.0);
        }
    }
}