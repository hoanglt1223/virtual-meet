// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod media;
mod devices;
mod recording;
mod scripting;
mod hotkeys;
mod commands;
mod commands_media;
mod commands_settings;
mod commands_hotkeys;
mod commands_scripting;
mod commands_recording;
mod error;
mod virtual;
mod audio;
mod audio_decoder;
mod audio_processor;
mod media_library;
mod metadata_extractor;
mod thumbnail_generator;
mod media_scanner;

use tauri::Manager;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize app state with core components
            let app_state = commands::init_state();
            app.manage(app_state);

            // Initialize virtual device state
            let virtual_device_state = commands::virtual_devices::VirtualDeviceState::default();
            app.manage(virtual_device_state);

            // Initialize hotkey state
            let hotkey_state = commands_hotkeys::init_hotkey_system();
            app.manage(hotkey_state);

            // Initialize scripting state
            let scripting_state = commands_scripting::init_scripting_system();
            app.manage(scripting_state);

            // Initialize recording system
            let mut app_handle = app.handle();
            if let Err(e) = commands_recording::initialize_recorder(&mut app_handle) {
                tracing::error!("Failed to initialize recorder: {}", e);
            } else {
                tracing::info!("Recording system initialized successfully");
            }

            // Initialize virtual webcam
            let state = app.state::<commands::AppState>();
            let webcam_state = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = webcam_state.webcam.initialize().await {
                    tracing::error!("Failed to initialize virtual webcam: {}", e);
                } else {
                    tracing::info!("Virtual webcam initialized successfully");
                }
            });

            // Initialize virtual microphone
            let mic_state = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = mic_state.microphone.initialize().await {
                    tracing::error!("Failed to initialize virtual microphone: {}", e);
                } else {
                    tracing::info!("Virtual microphone initialized successfully");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core device and virtual device commands
            commands::init_webcam,
            commands::start_streaming,
            commands::stop_streaming,
            commands::get_webcam_status,
            commands::list_video_devices,
            commands::validate_video_file,
            commands::init_microphone,
            commands::start_audio_streaming,
            commands::stop_audio_streaming,
            commands::get_microphone_status,
            commands::set_microphone_volume,
            commands::set_microphone_muted,
            commands::toggle_microphone_mute,
            commands::list_audio_devices,
            commands::validate_audio_file,
            commands::get_supported_audio_formats,
            commands::enumerate_all_devices,
            commands::enumerate_audio_devices,
            commands::enumerate_video_devices,
            commands::get_device_capabilities,
            commands::is_device_virtual,
            commands::get_virtual_devices,
            commands::get_physical_devices,
            commands::refresh_device_list,

            // Virtual device management commands
            commands::virtual_devices::initialize_webcam,
            commands::virtual_devices::initialize_microphone,
            commands::virtual_devices::start_webcam_streaming,
            commands::virtual_devices::start_microphone_streaming,
            commands::virtual_devices::stop_webcam_streaming,
            commands::virtual_devices::stop_microphone_streaming,
            commands::virtual_devices::get_virtual_device_status,
            commands::virtual_devices::list_virtual_devices,
            commands::virtual_devices::initialize_media_router,
            commands::virtual_devices::start_media_routing,
            commands::virtual_devices::stop_media_routing,
            commands::virtual_devices::switch_media,
            commands::virtual_devices::set_microphone_volume,
            commands::virtual_devices::get_microphone_volume,
            commands::virtual_devices::set_microphone_muted,
            commands::virtual_devices::get_microphone_muted,
            commands::virtual_devices::get_media_routing_status,
            commands::virtual_devices::get_webcam_video_info,
            commands::virtual_devices::get_webcam_buffer_status,
            commands::virtual_devices::get_microphone_buffer_status,

            // Media control commands
            commands_media::initialize_media_library,
            commands_media::load_media_library,
            commands_media::set_current_video,
            commands_media::set_current_audio,
            commands_media::get_supported_media_formats,
            commands_media::search_media_library,
            commands_media::search_media_library_enhanced,
            commands_media::get_media_library_status,
            commands_media::cleanup_media_library,

            // Recording commands (from commands_recording module)
            commands_recording::start_recording,
            commands_recording::stop_recording,
            commands_recording::get_recording_status,
            commands_recording::update_recording_config,
            commands_recording::get_recording_presets,
            commands_recording::test_recording_capabilities,

            // Settings commands
            commands_settings::get_settings,
            commands_settings::update_settings,
            commands_settings::reset_settings,
            commands_settings::export_settings,
            commands_settings::import_settings,
            commands_settings::get_available_video_devices,
            commands_settings::get_available_audio_devices,

            // Hotkey commands
            commands_hotkeys::register_hotkey,
            commands_hotkeys::unregister_hotkey,
            commands_hotkeys::get_registered_hotkeys,
            commands_hotkeys::get_hotkey_status,
            commands_hotkeys::set_hotkey_enabled,
            commands_hotkeys::execute_hotkey_action,
            commands_hotkeys::check_hotkey_conflicts,
            commands_hotkeys::get_default_hotkeys,

            // Scripting commands
            commands_scripting::execute_script,
            commands_scripting::create_script,
            commands_scripting::update_script,
            commands_scripting::delete_script,
            commands_scripting::get_scripts,
            commands_scripting::get_script,
            commands_scripting::validate_script,
            commands_scripting::get_script_templates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}