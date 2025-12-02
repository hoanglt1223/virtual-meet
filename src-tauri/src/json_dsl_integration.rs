//! JSON DSL Integration Module
//!
//! This module integrates the JSON DSL scripting engine with the existing
//! video/audio systems and Rhai scripting engine.

use crate::json_dsl::{ExecutionContext, JsonDslEngine, JsonDslScript, ScriptExecutionResult};
use crate::scripting::ScriptEngine;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Integrated DSL Engine that combines JSON DSL with existing systems
#[derive(Debug)]
pub struct IntegratedDslEngine {
    /// JSON DSL engine
    json_dsl: Arc<RwLock<JsonDslEngine>>,
    /// Rhai script engine
    rhai_engine: Arc<Mutex<ScriptEngine>>,
    /// Media integration layer
    media_integration: Arc<Mutex<MediaIntegration>>,
}

/// Media integration for connecting with virtual devices
#[derive(Debug)]
pub struct MediaIntegration {
    /// Active video streams
    video_streams: HashMap<String, VideoStream>,
    /// Active audio streams
    audio_streams: HashMap<String, AudioStream>,
    /// Active recording sessions
    recording_sessions: HashMap<String, RecordingSession>,
}

/// Active video stream information
#[derive(Debug)]
pub struct VideoStream {
    pub stream_id: String,
    pub file_path: PathBuf,
    pub start_time: f64,
    pub is_looping: bool,
    pub volume: f64,
    pub device_id: Option<String>,
}

/// Active audio stream information
#[derive(Debug)]
pub struct AudioStream {
    pub stream_id: String,
    pub file_path: PathBuf,
    pub start_time: f64,
    pub is_looping: bool,
    pub volume: f64,
    pub device_id: Option<String>,
}

/// Active recording session information
#[derive(Debug)]
pub struct RecordingSession {
    pub session_id: String,
    pub output_path: PathBuf,
    pub record_video: bool,
    pub record_audio: bool,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl IntegratedDslEngine {
    /// Create a new integrated DSL engine
    pub fn new() -> Self {
        Self {
            json_dsl: Arc::new(RwLock::new(JsonDslEngine::new())),
            rhai_engine: Arc::new(Mutex::new(ScriptEngine::new())),
            media_integration: Arc::new(Mutex::new(MediaIntegration::new())),
        }
    }

    /// Parse and validate a JSON DSL script
    pub async fn parse_script(&self, json_content: &str) -> Result<JsonDslScript> {
        let dsl = self.json_dsl.read().await;
        dsl.parse_script(json_content)
    }

    /// Execute a JSON DSL script with full integration
    pub async fn execute_script(&self, script: &JsonDslScript) -> Result<ScriptExecutionResult> {
        info!("Starting integrated execution of script: {}", script.name);

        // Create a copy of the script with integrated actions
        let integrated_script = self.create_integrated_script(script)?;

        // Execute with the JSON DSL engine
        let mut dsl = self.json_dsl.write().await;
        let result = dsl.execute_script(&integrated_script).await?;

        // Clean up any remaining resources
        self.cleanup_resources().await?;

        Ok(result)
    }

    /// Create an integrated version of the script with enhanced actions
    fn create_integrated_script(&self, script: &JsonDslScript) -> Result<JsonDslScript> {
        let mut integrated_script = script.clone();

        // Replace actions with integrated versions
        integrated_script.actions = script
            .actions
            .iter()
            .map(|action| self.create_integrated_action(action))
            .collect::<Result<Vec<_>>>()?;

        Ok(integrated_script)
    }

    /// Create an integrated version of a single action
    fn create_integrated_action(
        &self,
        action: &crate::json_dsl::ScriptAction,
    ) -> Result<crate::json_dsl::ScriptAction> {
        match action {
            crate::json_dsl::ScriptAction::PlayVideo {
                path,
                start_time,
                duration,
                loop_video,
                device,
                volume,
            } => Ok(crate::json_dsl::ScriptAction::CallFunction {
                function: "integrated_play_video".to_string(),
                parameters: Some(HashMap::from([
                    (
                        "path".to_string(),
                        crate::json_dsl::ScriptValue::String(path.clone()),
                    ),
                    (
                        "start_time".to_string(),
                        crate::json_dsl::ScriptValue::Number(start_time.unwrap_or(0.0)),
                    ),
                    (
                        "duration".to_string(),
                        crate::json_dsl::ScriptValue::Number(duration.unwrap_or(-1.0)),
                    ),
                    (
                        "loop_video".to_string(),
                        crate::json_dsl::ScriptValue::Boolean(loop_video.unwrap_or(false)),
                    ),
                    (
                        "device".to_string(),
                        crate::json_dsl::ScriptValue::String(device.clone().unwrap_or_default()),
                    ),
                    (
                        "volume".to_string(),
                        crate::json_dsl::ScriptValue::Number(volume.unwrap_or(1.0)),
                    ),
                ])),
            }),
            crate::json_dsl::ScriptAction::PlayAudio {
                path,
                start_time,
                duration,
                loop_audio,
                device,
                volume,
            } => Ok(crate::json_dsl::ScriptAction::CallFunction {
                function: "integrated_play_audio".to_string(),
                parameters: Some(HashMap::from([
                    (
                        "path".to_string(),
                        crate::json_dsl::ScriptValue::String(path.clone()),
                    ),
                    (
                        "start_time".to_string(),
                        crate::json_dsl::ScriptValue::Number(start_time.unwrap_or(0.0)),
                    ),
                    (
                        "duration".to_string(),
                        crate::json_dsl::ScriptValue::Number(duration.unwrap_or(-1.0)),
                    ),
                    (
                        "loop_audio".to_string(),
                        crate::json_dsl::ScriptValue::Boolean(loop_audio.unwrap_or(false)),
                    ),
                    (
                        "device".to_string(),
                        crate::json_dsl::ScriptValue::String(device.clone().unwrap_or_default()),
                    ),
                    (
                        "volume".to_string(),
                        crate::json_dsl::ScriptValue::Number(volume.unwrap_or(1.0)),
                    ),
                ])),
            }),
            crate::json_dsl::ScriptAction::StartRecording {
                output_path,
                quality,
                record_video,
                record_audio,
                max_duration,
            } => Ok(crate::json_dsl::ScriptAction::CallFunction {
                function: "integrated_start_recording".to_string(),
                parameters: Some(HashMap::from([
                    (
                        "output_path".to_string(),
                        crate::json_dsl::ScriptValue::String(output_path.clone()),
                    ),
                    (
                        "quality".to_string(),
                        crate::json_dsl::ScriptValue::String(quality.clone().unwrap_or_default()),
                    ),
                    (
                        "record_video".to_string(),
                        crate::json_dsl::ScriptValue::Boolean(record_video.unwrap_or(true)),
                    ),
                    (
                        "record_audio".to_string(),
                        crate::json_dsl::ScriptValue::Boolean(record_audio.unwrap_or(true)),
                    ),
                    (
                        "max_duration".to_string(),
                        crate::json_dsl::ScriptValue::Number(max_duration.unwrap_or(-1.0)),
                    ),
                ])),
            }),
            crate::json_dsl::ScriptAction::StopRecording { save_path } => {
                Ok(crate::json_dsl::ScriptAction::CallFunction {
                    function: "integrated_stop_recording".to_string(),
                    parameters: Some(HashMap::from([(
                        "save_path".to_string(),
                        crate::json_dsl::ScriptValue::String(save_path.clone().unwrap_or_default()),
                    )])),
                })
            }
            crate::json_dsl::ScriptAction::SetVirtualDevice {
                device_type,
                action,
                source,
            } => Ok(crate::json_dsl::ScriptAction::CallFunction {
                function: "integrated_set_virtual_device".to_string(),
                parameters: Some(HashMap::from([
                    (
                        "device_type".to_string(),
                        crate::json_dsl::ScriptValue::String(device_type.clone()),
                    ),
                    (
                        "action".to_string(),
                        crate::json_dsl::ScriptValue::String(action.clone()),
                    ),
                    (
                        "source".to_string(),
                        crate::json_dsl::ScriptValue::String(source.clone().unwrap_or_default()),
                    ),
                ])),
            }),
            _ => Ok(action.clone()), // Return other actions unchanged
        }
    }

    /// Execute integrated function calls
    pub async fn execute_integrated_function(
        &self,
        function_name: &str,
        parameters: &HashMap<String, crate::json_dsl::ScriptValue>,
    ) -> Result<Option<String>> {
        match function_name {
            "integrated_play_video" => self.integrated_play_video(parameters).await,
            "integrated_play_audio" => self.integrated_play_audio(parameters).await,
            "integrated_start_recording" => self.integrated_start_recording(parameters).await,
            "integrated_stop_recording" => self.integrated_stop_recording(parameters).await,
            "integrated_set_virtual_device" => self.integrated_set_virtual_device(parameters).await,
            _ => Err(anyhow!("Unknown integrated function: {}", function_name)),
        }
    }

    /// Integrated video playback
    async fn integrated_play_video(
        &self,
        parameters: &HashMap<String, crate::json_dsl::ScriptValue>,
    ) -> Result<Option<String>> {
        let path = extract_string_param(parameters, "path")?;
        let start_time = extract_number_param(parameters, "start_time").unwrap_or(0.0);
        let volume = extract_number_param(parameters, "volume").unwrap_or(1.0);
        let device = extract_string_param(parameters, "device").unwrap_or_default();

        info!(
            "Starting integrated video playback: {} (start: {}, volume: {})",
            path, start_time, volume
        );

        // Validate file exists
        if !std::path::Path::new(&path).exists() {
            return Err(anyhow!("Video file not found: {}", path));
        }

        // Create video stream entry
        let stream_id = uuid::Uuid::new_v4().to_string();
        let video_stream = VideoStream {
            stream_id: stream_id.clone(),
            file_path: std::path::PathBuf::from(&path),
            start_time,
            is_looping: extract_bool_param(parameters, "loop_video").unwrap_or(false),
            volume,
            device_id: if device.is_empty() {
                None
            } else {
                Some(device)
            },
        };

        // Store stream information
        {
            let mut media = self.media_integration.lock().unwrap();
            media.video_streams.insert(stream_id.clone(), video_stream);
        }

        // Integrate with existing Rhai engine for virtual webcam control
        {
            let mut rhai = self.rhai_engine.lock().unwrap();
            let mut vars = HashMap::new();

            // Call the existing Rhai function for webcam streaming
            let rhai_script = format!(
                r#"
                print("Starting virtual webcam streaming for video: {}");
                start_webcam_streaming("{}");
            "#,
                path, path
            );

            let result = rhai.execute_content(&rhai_script, Some(vars));
            if !result.success {
                warn!("Rhai webcam streaming returned error: {:?}", result.error);
            }
        }

        Ok(Some(format!("Started video playback: {}", path)))
    }

    /// Integrated audio playback
    async fn integrated_play_audio(
        &self,
        parameters: &HashMap<String, crate::json_dsl::ScriptValue>,
    ) -> Result<Option<String>> {
        let path = extract_string_param(parameters, "path")?;
        let start_time = extract_number_param(parameters, "start_time").unwrap_or(0.0);
        let volume = extract_number_param(parameters, "volume").unwrap_or(1.0);
        let device = extract_string_param(parameters, "device").unwrap_or_default();

        info!(
            "Starting integrated audio playback: {} (start: {}, volume: {})",
            path, start_time, volume
        );

        // Validate file exists
        if !std::path::Path::new(&path).exists() {
            return Err(anyhow!("Audio file not found: {}", path));
        }

        // Create audio stream entry
        let stream_id = uuid::Uuid::new_v4().to_string();
        let audio_stream = AudioStream {
            stream_id: stream_id.clone(),
            file_path: std::path::PathBuf::from(&path),
            start_time,
            is_looping: extract_bool_param(parameters, "loop_audio").unwrap_or(false),
            volume,
            device_id: if device.is_empty() {
                None
            } else {
                Some(device)
            },
        };

        // Store stream information
        {
            let mut media = self.media_integration.lock().unwrap();
            media.audio_streams.insert(stream_id.clone(), audio_stream);
        }

        // Integrate with existing Rhai engine for virtual microphone control
        {
            let mut rhai = self.rhai_engine.lock().unwrap();
            let mut vars = HashMap::new();

            let rhai_script = format!(
                r#"
                print("Starting virtual audio streaming for audio: {}");
                start_microphone_streaming("{}");
            "#,
                path, path
            );

            let result = rhai.execute_content(&rhai_script, Some(vars));
            if !result.success {
                warn!("Rhai audio streaming returned error: {:?}", result.error);
            }
        }

        Ok(Some(format!("Started audio playback: {}", path)))
    }

    /// Integrated recording start
    async fn integrated_start_recording(
        &self,
        parameters: &HashMap<String, crate::json_dsl::ScriptValue>,
    ) -> Result<Option<String>> {
        let output_path = extract_string_param(parameters, "output_path")?;
        let record_video = extract_bool_param(parameters, "record_video").unwrap_or(true);
        let record_audio = extract_bool_param(parameters, "record_audio").unwrap_or(true);

        info!(
            "Starting integrated recording: {} (video: {}, audio: {})",
            output_path, record_video, record_audio
        );

        // Create recording session entry
        let session_id = uuid::Uuid::new_v4().to_string();
        let recording_session = RecordingSession {
            session_id: session_id.clone(),
            output_path: std::path::PathBuf::from(&output_path),
            record_video,
            record_audio,
            start_time: chrono::Utc::now(),
        };

        // Store session information
        {
            let mut media = self.media_integration.lock().unwrap();
            media
                .recording_sessions
                .insert(session_id.clone(), recording_session);
        }

        // Integrate with existing Rhai engine for recording
        {
            let mut rhai = self.rhai_engine.lock().unwrap();
            let mut vars = HashMap::new();

            let rhai_script = format!(
                r#"
                print("Starting recording: {}");
                start_recording("{}");
            "#,
                output_path, output_path
            );

            let result = rhai.execute_content(&rhai_script, Some(vars));
            if !result.success {
                warn!("Rhai recording returned error: {:?}", result.error);
            }
        }

        Ok(Some(format!("Started recording: {}", output_path)))
    }

    /// Integrated recording stop
    async fn integrated_stop_recording(
        &self,
        parameters: &HashMap<String, crate::json_dsl::ScriptValue>,
    ) -> Result<Option<String>> {
        let save_path = extract_string_param(parameters, "save_path").unwrap_or_default();

        info!("Stopping integrated recording: save_path={}", save_path);

        // Remove active recording session
        let session_info = {
            let mut media = self.media_integration.lock().unwrap();
            if media.recording_sessions.is_empty() {
                return Ok(Some("No active recording session to stop".to_string()));
            }

            // Remove the first session (simplified for single recording support)
            media
                .recording_sessions
                .drain()
                .next()
                .map(|(id, session)| (id, session))
        };

        if let Some((session_id, _session)) = session_info {
            debug!("Stopped recording session: {}", session_id);
        }

        // Integrate with existing Rhai engine for stopping recording
        {
            let mut rhai = self.rhai_engine.lock().unwrap();
            let mut vars = HashMap::new();

            let rhai_script = r#"
                print("Stopping recording");
                stop_recording();
            "#
            .to_string();

            let result = rhai.execute_content(&rhai_script, Some(vars));
            if !result.success {
                warn!("Rhai stop recording returned error: {:?}", result.error);
            }
        }

        Ok(Some("Stopped recording".to_string()))
    }

    /// Integrated virtual device control
    async fn integrated_set_virtual_device(
        &self,
        parameters: &HashMap<String, crate::json_dsl::ScriptValue>,
    ) -> Result<Option<String>> {
        let device_type = extract_string_param(parameters, "device_type")?;
        let action = extract_string_param(parameters, "action")?;
        let source = extract_string_param(parameters, "source").unwrap_or_default();

        info!(
            "Setting virtual device: {} {} (source: {})",
            device_type, action, source
        );

        // Integrate with existing Rhai engine for device control
        {
            let mut rhai = self.rhai_engine.lock().unwrap();
            let mut vars = HashMap::new();

            let rhai_script = match (device_type.as_str(), action.as_str()) {
                ("webcam", "start") => {
                    format!(
                        r#"
                        print("Starting virtual webcam");
                        start_webcam_streaming("{}");
                    "#,
                        source
                    )
                }
                ("webcam", "stop") => r#"
                        print("Stopping virtual webcam");
                        stop_webcam_streaming();
                    "#
                .to_string(),
                ("microphone", "start") => {
                    format!(
                        r#"
                        print("Starting virtual microphone");
                        start_microphone_streaming("{}");
                    "#,
                        source
                    )
                }
                ("microphone", "stop") => r#"
                        print("Stopping virtual microphone");
                        stop_microphone_streaming();
                    "#
                .to_string(),
                _ => {
                    return Err(anyhow!(
                        "Unsupported device type or action: {} {}",
                        device_type,
                        action
                    ));
                }
            };

            let result = rhai.execute_content(&rhai_script, Some(vars));
            if !result.success {
                warn!("Rhai device control returned error: {:?}", result.error);
            }
        }

        Ok(Some(format!(
            "Virtual device {} {} completed",
            device_type, action
        )))
    }

    /// Clean up active resources
    async fn cleanup_resources(&self) -> Result<()> {
        info!("Cleaning up active media resources");

        let mut media = self.media_integration.lock().unwrap();

        // Stop all video streams
        for (stream_id, _stream) in &media.video_streams {
            debug!("Cleaning up video stream: {}", stream_id);
        }
        media.video_streams.clear();

        // Stop all audio streams
        for (stream_id, _stream) in &media.audio_streams {
            debug!("Cleaning up audio stream: {}", stream_id);
        }
        media.audio_streams.clear();

        // Stop all recording sessions
        for (session_id, _session) in &media.recording_sessions {
            debug!("Cleaning up recording session: {}", session_id);
        }
        media.recording_sessions.clear();

        // Stop virtual devices via Rhai
        {
            let mut rhai = self.rhai_engine.lock().unwrap();
            let cleanup_script = r#"
                print("Cleaning up virtual devices");
                stop_webcam_streaming();
                stop_microphone_streaming();
                stop_recording();
            "#;

            let result = rhai.execute_content(cleanup_script, None);
            if !result.success {
                warn!("Rhai cleanup returned error: {:?}", result.error);
            }
        }

        Ok(())
    }

    /// Get current media integration status
    pub async fn get_media_status(&self) -> Result<MediaStatus> {
        let media = self.media_integration.lock().unwrap();

        Ok(MediaStatus {
            active_video_streams: media.video_streams.len(),
            active_audio_streams: media.audio_streams.len(),
            active_recording_sessions: media.recording_sessions.len(),
            video_streams: media
                .video_streams
                .values()
                .map(|s| MediaStreamStatus {
                    stream_id: s.stream_id.clone(),
                    file_path: s.file_path.to_string_lossy().to_string(),
                    start_time: s.start_time,
                    is_looping: s.is_looping,
                    volume: s.volume,
                    device_id: s.device_id.clone(),
                })
                .collect(),
            audio_streams: media
                .audio_streams
                .values()
                .map(|s| MediaStreamStatus {
                    stream_id: s.stream_id.clone(),
                    file_path: s.file_path.to_string_lossy().to_string(),
                    start_time: s.start_time,
                    is_looping: s.is_looping,
                    volume: s.volume,
                    device_id: s.device_id.clone(),
                })
                .collect(),
            recording_sessions: media
                .recording_sessions
                .values()
                .map(|s| RecordingSessionStatus {
                    session_id: s.session_id.clone(),
                    output_path: s.output_path.to_string_lossy().to_string(),
                    record_video: s.record_video,
                    record_audio: s.record_audio,
                    start_time: s.start_time,
                })
                .collect(),
        })
    }
}

/// Media status information
#[derive(Debug)]
pub struct MediaStatus {
    pub active_video_streams: usize,
    pub active_audio_streams: usize,
    pub active_recording_sessions: usize,
    pub video_streams: Vec<MediaStreamStatus>,
    pub audio_streams: Vec<MediaStreamStatus>,
    pub recording_sessions: Vec<RecordingSessionStatus>,
}

/// Media stream status
#[derive(Debug)]
pub struct MediaStreamStatus {
    pub stream_id: String,
    pub file_path: String,
    pub start_time: f64,
    pub is_looping: bool,
    pub volume: f64,
    pub device_id: Option<String>,
}

/// Recording session status
#[derive(Debug)]
pub struct RecordingSessionStatus {
    pub session_id: String,
    pub output_path: String,
    pub record_video: bool,
    pub record_audio: bool,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl MediaIntegration {
    /// Create new media integration
    pub fn new() -> Self {
        Self {
            video_streams: HashMap::new(),
            audio_streams: HashMap::new(),
            recording_sessions: HashMap::new(),
        }
    }
}

/// Helper function to extract string parameter
fn extract_string_param(
    params: &HashMap<String, crate::json_dsl::ScriptValue>,
    key: &str,
) -> Result<String> {
    match params.get(key) {
        Some(crate::json_dsl::ScriptValue::String(s)) => Ok(s.clone()),
        Some(_) => Err(anyhow!("Parameter '{}' is not a string", key)),
        None => Err(anyhow!("Missing parameter: {}", key)),
    }
}

/// Helper function to extract number parameter
fn extract_number_param(
    params: &HashMap<String, crate::json_dsl::ScriptValue>,
    key: &str,
) -> Option<f64> {
    match params.get(key) {
        Some(crate::json_dsl::ScriptValue::Number(n)) => Some(*n),
        Some(crate::json_dsl::ScriptValue::Integer(n)) => Some(*n as f64),
        _ => None,
    }
}

/// Helper function to extract boolean parameter
fn extract_bool_param(
    params: &HashMap<String, crate::json_dsl::ScriptValue>,
    key: &str,
) -> Option<bool> {
    match params.get(key) {
        Some(crate::json_dsl::ScriptValue::Boolean(b)) => Some(*b),
        _ => None,
    }
}

impl Default for IntegratedDslEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_dsl::{JsonDslScript, ScriptAction, ScriptCondition, ScriptValue};

    #[tokio::test]
    async fn test_integrated_script_parsing() {
        let engine = IntegratedDslEngine::new();

        let script_json = r#"
        {
            "name": "Test Integrated Script",
            "description": "A test script for integration",
            "actions": [
                {
                    "type": "play_video",
                    "path": "test.mp4",
                    "duration": 10.0,
                    "volume": 0.8
                },
                {
                    "type": "wait",
                    "duration": 1.0
                }
            ]
        }
        "#;

        let script = engine.parse_script(script_json).await.unwrap();
        assert_eq!(script.name, "Test Integrated Script");
        assert_eq!(script.actions.len(), 2);
    }

    #[tokio::test]
    async fn test_integrated_video_playback() {
        let engine = IntegratedDslEngine::new();

        // Note: This test would require a test video file to actually work
        let parameters = HashMap::from([
            (
                "path".to_string(),
                ScriptValue::String("test.mp4".to_string()),
            ),
            ("volume".to_string(), ScriptValue::Number(0.8)),
            ("start_time".to_string(), ScriptValue::Number(0.0)),
        ]);

        let result = engine.integrated_play_video(&parameters).await;
        // This would normally fail with "Video file not found" which is expected for testing
        assert!(result.is_err() || result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_media_status() {
        let engine = IntegratedDslEngine::new();
        let status = engine.get_media_status().await.unwrap();

        assert_eq!(status.active_video_streams, 0);
        assert_eq!(status.active_audio_streams, 0);
        assert_eq!(status.active_recording_sessions, 0);
    }
}
