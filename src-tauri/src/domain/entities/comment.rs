//! Comment — komentar galeri (port `Comment` kuron_core mobile).
//! Sumber: nhentai `?include=comments` (embedded di respons detail).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Satu komentar: penulis + isi + avatar + waktu (epoch detik).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Comment {
    pub id: String,
    pub username: String,
    /// Isi mentah (bisa HTML ala mobile) — UI render sebagai TEKS.
    pub body: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub post_date: Option<i64>,
}
