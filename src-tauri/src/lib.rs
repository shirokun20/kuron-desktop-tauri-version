pub mod application;
pub mod cache;
pub mod commands;
pub mod core;
pub mod data;
pub mod domain;
pub mod network;

use commands::{cmd_app_info, cmd_hello_world, cmd_home_feed};
use core::{logger, AppState};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            cmd_hello_world,
            cmd_app_info,
            cmd_home_feed
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
