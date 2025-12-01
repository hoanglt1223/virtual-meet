// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod media;
mod devices;
mod recording;
mod scripting;
mod hotkeys;
mod commands;
mod error;
mod virtual;

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
            // Initialize app state with video pipeline
            let app_state = commands::init_state();
            app.manage(app_state);

            // Initialize virtual webcam
            let state = app.state::<commands::AppState>();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = state.webcam.initialize().await {
                    tracing::error!("Failed to initialize virtual webcam: {}", e);
                } else {
                    tracing::info!("Virtual webcam initialized successfully");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Video pipeline commands
            commands::init_webcam,
            commands::start_streaming,
            commands::stop_streaming,
            commands::get_webcam_status,
            commands::list_video_devices,
            commands::validate_video_file,
            // Legacy commands (keep for compatibility)
            commands::get_video_devices,
            commands::get_audio_devices,
            commands::load_media_library,
            commands::set_current_video,
            commands::set_current_audio,
            commands::start_recording,
            commands::stop_recording,
            commands::register_hotkey,
            commands::execute_script,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}