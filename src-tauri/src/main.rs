// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod media;
mod devices;
mod recording;
mod scripting;
mod hotkeys;
mod commands;
mod error;

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
            // Initialize app state and services here
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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