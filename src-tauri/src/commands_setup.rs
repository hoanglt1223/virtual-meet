//! Setup and device detection commands
//! Detects installed virtual cameras (OBS, ManyCam) and audio cables (VB-CABLE, VoiceMeeter)

use crate::virtual_device::imf_webcam::ImfWebcam;
use serde::Serialize;
use std::process::{Command, Stdio};
use tauri::command;
use tracing::info;

/// Detected virtual device setup status
#[derive(Debug, Clone, Serialize)]
pub struct VirtualDeviceSetup {
    /// Available virtual cameras (name list)
    pub virtual_cameras: Vec<String>,
    /// Available virtual audio cables (output devices that act as mic input)
    pub virtual_audio_cables: Vec<String>,
    /// Whether ffmpeg CLI is available
    pub ffmpeg_available: bool,
    /// Whether OBS Virtual Camera is detected
    pub obs_camera_detected: bool,
    /// Whether VB-CABLE is detected
    pub vb_cable_detected: bool,
    /// Whether VoiceMeeter is detected
    pub voicemeeter_detected: bool,
    /// OS build number (for IMFVirtualCamera support check)
    pub windows_build: u32,
    /// Whether IMFVirtualCamera API is available (Win11 22000+)
    pub imf_virtual_camera_supported: bool,
    /// Recommendations for user
    pub recommendations: Vec<SetupRecommendation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupRecommendation {
    pub category: String,       // "video" or "audio"
    pub status: String,         // "ready", "missing", "optional"
    pub message: String,
    pub action_url: Option<String>,
}

#[command]
pub async fn detect_virtual_devices() -> Result<VirtualDeviceSetup, String> {
    info!("Detecting virtual devices...");

    // 1. Check ffmpeg availability
    let ffmpeg_available = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // 2. List video devices via ffmpeg -list_devices
    let virtual_cameras = if ffmpeg_available {
        detect_virtual_cameras().unwrap_or_default()
    } else {
        Vec::new()
    };

    // 3. List audio output devices via cpal, filter for virtual cables
    let virtual_audio_cables = detect_virtual_audio_cables();

    // 4. Check specific devices
    let obs_camera_detected = virtual_cameras.iter()
        .any(|d| d.to_lowercase().contains("obs"));
    let vb_cable_detected = virtual_audio_cables.iter()
        .any(|d| d.to_lowercase().contains("cable") || d.to_lowercase().contains("vb-audio"));
    let voicemeeter_detected = virtual_audio_cables.iter()
        .any(|d| d.to_lowercase().contains("voicemeeter"));

    // 5. Check Windows build for IMFVirtualCamera support
    let windows_build = get_windows_build();
    let imf_virtual_camera_supported = windows_build >= 22000;

    // 6. Build recommendations
    let mut recommendations = Vec::new();

    // Video recommendations
    if virtual_cameras.is_empty() {
        if imf_virtual_camera_supported {
            recommendations.push(SetupRecommendation {
                category: "video".to_string(),
                status: "optional".to_string(),
                message: "No virtual camera found. You can install OBS Virtual Camera, or use the built-in virtual camera (coming soon).".to_string(),
                action_url: Some("https://obsproject.com/download".to_string()),
            });
        } else {
            recommendations.push(SetupRecommendation {
                category: "video".to_string(),
                status: "missing".to_string(),
                message: "No virtual camera found. Install OBS Studio to get OBS Virtual Camera.".to_string(),
                action_url: Some("https://obsproject.com/download".to_string()),
            });
        }
    } else {
        recommendations.push(SetupRecommendation {
            category: "video".to_string(),
            status: "ready".to_string(),
            message: format!("Virtual camera ready: {}", virtual_cameras.join(", ")),
            action_url: None,
        });
    }

    // Audio recommendations
    if !vb_cable_detected && !voicemeeter_detected {
        recommendations.push(SetupRecommendation {
            category: "audio".to_string(),
            status: "missing".to_string(),
            message: "No virtual audio cable found. Install VB-CABLE to use audio as microphone.".to_string(),
            action_url: Some("https://vb-audio.com/Cable/".to_string()),
        });
    } else {
        let cable_name = if vb_cable_detected { "VB-CABLE" } else { "VoiceMeeter" };
        recommendations.push(SetupRecommendation {
            category: "audio".to_string(),
            status: "ready".to_string(),
            message: format!("{} detected. Audio-as-microphone ready.", cable_name),
            action_url: None,
        });
    }

    // ffmpeg recommendation
    if !ffmpeg_available {
        recommendations.push(SetupRecommendation {
            category: "video".to_string(),
            status: "missing".to_string(),
            message: "FFmpeg not found. Required for video streaming.".to_string(),
            action_url: Some("https://ffmpeg.org/download.html".to_string()),
        });
    }

    let setup = VirtualDeviceSetup {
        virtual_cameras,
        virtual_audio_cables,
        ffmpeg_available,
        obs_camera_detected,
        vb_cable_detected,
        voicemeeter_detected,
        windows_build,
        imf_virtual_camera_supported,
        recommendations,
    };

    info!("Device detection complete: obs={}, vb_cable={}, voicemeeter={}, imf_supported={}",
        setup.obs_camera_detected, setup.vb_cable_detected, setup.voicemeeter_detected, setup.imf_virtual_camera_supported);

    Ok(setup)
}

/// Detect virtual cameras via ffmpeg device listing
fn detect_virtual_cameras() -> Result<Vec<String>, String> {
    let output = Command::new("ffmpeg")
        .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut cameras = Vec::new();
    let mut in_video = false;

    for line in stderr.lines() {
        if line.contains("DirectShow video devices") {
            in_video = true;
            continue;
        }
        if line.contains("DirectShow audio devices") {
            in_video = false;
            continue;
        }
        if in_video {
            if let Some(start) = line.find('"') {
                if let Some(end) = line.rfind('"') {
                    if end > start {
                        let name = &line[start + 1..end];
                        // Filter for virtual cameras (skip "Alternative name" lines)
                        if !name.contains("Alternative name") {
                            // Check if it's a virtual camera by name patterns
                            let lower = name.to_lowercase();
                            if lower.contains("virtual") || lower.contains("obs")
                                || lower.contains("manycam") || lower.contains("snap")
                                || lower.contains("xsplit") || lower.contains("droidcam") {
                                cameras.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(cameras)
}

/// Detect virtual audio cables from cpal output devices
fn detect_virtual_audio_cables() -> Vec<String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let mut cables = Vec::new();

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                let lower = name.to_lowercase();
                if lower.contains("cable") || lower.contains("vb-audio")
                    || lower.contains("voicemeeter") || lower.contains("virtual")
                    || lower.contains("blackhole") {
                    cables.push(name);
                }
            }
        }
    } else {
        tracing::warn!("Failed to enumerate output devices for virtual audio cable detection");
    }

    cables
}

// ---------------------------------------------------------------------------
// Webcam mode enumeration
// ---------------------------------------------------------------------------

/// Info about a single available webcam backend mode
#[derive(Debug, Clone, Serialize)]
pub struct WebcamModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Whether this mode is fully ready to use right now
    pub available: bool,
    /// Human-readable list of prerequisites
    pub requires: Vec<String>,
}

/// Return the available webcam backend modes for this machine.
#[command]
pub async fn get_webcam_modes() -> Result<Vec<WebcamModeInfo>, String> {
    let mut modes = vec![
        WebcamModeInfo {
            id: "obs".to_string(),
            name: "OBS Virtual Camera".to_string(),
            description: "Stream via ffmpeg to OBS Virtual Camera or any compatible \
                           DirectShow virtual camera device."
                .to_string(),
            // OBS mode is available as long as ffmpeg is on PATH
            available: Command::new("ffmpeg")
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
            requires: vec![
                "ffmpeg".to_string(),
                "OBS Virtual Camera driver".to_string(),
            ],
        },
    ];

    if ImfWebcam::is_available() {
        modes.push(WebcamModeInfo {
            id: "imf".to_string(),
            name: "Built-in Virtual Camera".to_string(),
            description: "Windows 11 built-in virtual camera via IMFVirtualCamera. \
                           No OBS installation required."
                .to_string(),
            available: ImfWebcam::is_com_source_registered(),
            requires: vec![
                "Windows 11 (build 22000+)".to_string(),
                "VirtualMeet Camera Driver (COM DLL)".to_string(),
            ],
        });
    }

    Ok(modes)
}

// ---------------------------------------------------------------------------
// COM DLL registration for built-in virtual camera
// ---------------------------------------------------------------------------

/// Register the vcam-source COM DLL with regsvr32 (requires admin)
#[command]
pub async fn register_vcam_driver() -> Result<String, String> {
    info!("Registering vcam-source COM DLL...");

    // Find the DLL next to the app executable
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("No parent directory")?
        .to_path_buf();

    let dll_path = exe_dir.join("vcam_source.dll");
    if !dll_path.exists() {
        return Err(format!("vcam_source.dll not found at: {}", dll_path.display()));
    }

    // Run regsvr32 (requires admin elevation)
    let status = Command::new("regsvr32")
        .args(["/s", &dll_path.to_string_lossy()])
        .status()
        .map_err(|e| format!("Failed to run regsvr32: {}", e))?;

    if status.success() {
        info!("vcam-source COM DLL registered successfully");
        Ok("Virtual camera driver registered successfully. You may need to restart the app.".to_string())
    } else {
        Err("Registration failed. Make sure to run as administrator.".to_string())
    }
}

/// Unregister the vcam-source COM DLL
#[command]
pub async fn unregister_vcam_driver() -> Result<String, String> {
    info!("Unregistering vcam-source COM DLL...");

    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("No parent directory")?
        .to_path_buf();

    let dll_path = exe_dir.join("vcam_source.dll");

    let status = Command::new("regsvr32")
        .args(["/s", "/u", &dll_path.to_string_lossy()])
        .status()
        .map_err(|e| format!("Failed to run regsvr32: {}", e))?;

    if status.success() {
        info!("vcam-source COM DLL unregistered");
        Ok("Virtual camera driver unregistered.".to_string())
    } else {
        Err("Unregistration failed. Make sure to run as administrator.".to_string())
    }
}

// ---------------------------------------------------------------------------

/// Get Windows build number
fn get_windows_build() -> u32 {
    let output = Command::new("cmd")
        .args(["/c", "ver"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok();

    if let Some(output) = output {
        let ver_str = String::from_utf8_lossy(&output.stdout);
        // Parse "Microsoft Windows [Version 10.0.22631.xxxx]"
        if let Some(bracket_start) = ver_str.find('[') {
            if let Some(bracket_end) = ver_str.find(']') {
                let version_part = &ver_str[bracket_start + 1..bracket_end];
                // Get the build number (third number in "10.0.XXXXX")
                let parts: Vec<&str> = version_part.split('.').collect();
                if parts.len() >= 3 {
                    if let Ok(build) = parts[2].parse::<u32>() {
                        return build;
                    }
                }
            }
        }
    }

    0
}
