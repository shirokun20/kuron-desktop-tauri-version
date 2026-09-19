//! Hello commands: Frontend -> Rust (spec §15).

use crate::{application::SayHelloUseCase, domain::Hello};

#[tauri::command]
pub fn cmd_hello_world(name: Option<String>) -> Result<Hello, String> {
    SayHelloUseCase::new()
        .execute(name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_app_info() -> AppInfo {
    AppInfo {
        name: "Kuron Desktop".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        backend: "tauri-v2".to_string(),
    }
}

#[derive(serde::Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub backend: String,
}
