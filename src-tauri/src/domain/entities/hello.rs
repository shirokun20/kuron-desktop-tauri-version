//! Hello entity: pure Domain, serde only for IPC transport.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Hello {
    pub name: String,
    pub message: String,
}

impl Hello {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        let name = if name.is_empty() {
            "Kuron".to_string()
        } else {
            name
        };
        Self {
            message: format!("Halo, {name}! Kuron Desktop (Tauri v2) jalan."),
            name,
        }
    }
}
