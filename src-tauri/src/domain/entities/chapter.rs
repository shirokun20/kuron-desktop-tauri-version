//! Chapter entity — port `entities/chapter.dart`.
//! `isExternal/isReadableInApp` logic pindah Rust (Fase 1).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub content_id: String,
    pub title: String,
    pub order: u32,
    pub is_external: bool,
    pub external_url: Option<String>,
}
