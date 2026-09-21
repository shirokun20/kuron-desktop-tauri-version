//! HistoryEntry — satu baris riwayat baca (subset `History` mobile).
//! Port minimal `entities/history.dart` untuk 8.3: `contentId` + halaman
//! terakhir + waktu. Field kaya mobile (`totalPages`, `timeSpent`,
//! `isCompleted`, chapter-*) nyusul bareng reader Fase 5.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Satu entri riwayat: konten + posisi baca terakhir.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HistoryEntry {
    pub content_id: String,
    /// Halaman terakhir dibaca (1-based, ala `History.lastPage` mobile).
    pub position: i64,
    /// Terakhir dilihat, unix epoch detik (ala `History.lastViewed`).
    pub updated_at: i64,
}
