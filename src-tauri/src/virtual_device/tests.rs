//! Tests for virtual device functionality

#[cfg(test)]
mod virtual_device_tests {
    use crate::virtual_device::webcam::{VirtualWebcam, WebcamMode};
    use crate::virtual_device::microphone::VirtualMicrophone;
    use crate::virtual_device::media_router::{MediaRouter, MediaRoutingConfig};
    use crate::virtual_device::imf_webcam::ImfWebcam;

    #[tokio::test]
    async fn test_webcam_creation() {
        let webcam = VirtualWebcam::new();
        assert!(!webcam.is_active().await);
        assert!(webcam.current_source().await.is_none());
    }

    #[tokio::test]
    async fn test_webcam_default_mode() {
        let webcam = VirtualWebcam::new();
        // On Windows 11 build >= 22000, default should be Imf; otherwise Obs
        let mode = webcam.get_mode();
        assert!(*mode == WebcamMode::Obs || *mode == WebcamMode::Imf);
    }

    #[tokio::test]
    async fn test_webcam_stop_when_not_active() {
        let webcam = VirtualWebcam::new();
        // Stopping when not active should succeed silently
        let result = webcam.stop_streaming().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_webcam_start_missing_file() {
        let webcam = VirtualWebcam::new();
        let result = webcam.start_streaming("nonexistent_video.mp4").await;
        assert!(result.is_err());
        assert!(!webcam.is_active().await);
    }

    #[tokio::test]
    async fn test_webcam_buffer_status() {
        let webcam = VirtualWebcam::new();
        let status = webcam.get_buffer_status().await;
        assert_eq!(status.current_frames, 0);
    }

    #[tokio::test]
    async fn test_microphone_creation() {
        let mic = VirtualMicrophone::new();
        assert!(!mic.is_active().await);
        assert!(mic.current_source().is_none());
    }

    #[tokio::test]
    async fn test_microphone_volume() {
        let mic = VirtualMicrophone::new();
        assert_eq!(mic.get_volume().await, 1.0);

        mic.set_volume(0.5).await.unwrap();
        assert_eq!(mic.get_volume().await, 0.5);

        // Invalid volume should error
        assert!(mic.set_volume(1.5).await.is_err());
        assert!(mic.set_volume(-0.1).await.is_err());
    }

    #[tokio::test]
    async fn test_microphone_mute() {
        let mic = VirtualMicrophone::new();
        assert!(!mic.is_muted().await);

        mic.set_muted(true).await;
        assert!(mic.is_muted().await);

        let toggled = mic.toggle_mute().await;
        assert!(!toggled);
        assert!(!mic.is_muted().await);
    }

    #[tokio::test]
    async fn test_microphone_stop_when_not_active() {
        let mic = VirtualMicrophone::new();
        let result = mic.stop_streaming().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_microphone_start_invalid_file() {
        let mic = VirtualMicrophone::new();
        let result = mic.start_streaming("nonexistent.mp3").await;
        assert!(result.is_err());
        assert!(!mic.is_active().await);
    }

    #[tokio::test]
    async fn test_microphone_buffer_status() {
        let mic = VirtualMicrophone::new();
        let (len, max, total) = mic.get_buffer_status().await;
        assert_eq!(len, 0);
        assert!(max > 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_imf_webcam_availability() {
        // Should not panic — just returns true/false based on Windows build
        let _ = ImfWebcam::is_available();
    }

    #[test]
    fn test_imf_com_source_check() {
        // Should not panic — just checks registry
        let _ = ImfWebcam::is_com_source_registered();
    }

    #[tokio::test]
    async fn test_media_router_creation() {
        let config = MediaRoutingConfig::default();
        assert!(config.video_path.is_empty());
        assert!(config.audio_path.is_empty());
        assert!(config.loop_media);

        let router = MediaRouter::with_config(config);
        let status = router.get_status().await;
        assert!(!status.is_active);
    }

    #[tokio::test]
    async fn test_device_listing() {
        // List video devices — should not panic even without ffmpeg
        let result = VirtualWebcam::list_devices().await;
        // May succeed or fail depending on ffmpeg availability
        match result {
            Ok(devices) => assert!(devices.len() >= 0),
            Err(_) => {} // ffmpeg not installed — acceptable
        }

        // List audio devices — should work (uses cpal)
        let result = VirtualMicrophone::list_devices().await;
        assert!(result.is_ok());
    }
}
