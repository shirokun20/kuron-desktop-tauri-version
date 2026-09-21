//! HistoryItem — satu baris riwayat siap-display (subset `History` mobile).
//! Denormalisasi seperti mobile: posisi baca + snapshot display
//! (`title`/`coverUrl`/`sourceId`) agar daftar tak perlu JOIN susulan.
//! Field kaya mobile (`totalPages`, `timeSpent`, `isCompleted`, chapter-*)
//! nyusul bareng reader Fase 5.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Satu entri riwayat: konten + posisi baca terakhir + waktu.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HistoryItem {
    pub content_id: String,
    pub title: String,
    pub cover_url: String,
    pub source_id: String,
    /// Halaman terakhir dibaca (1-based, ala `History.lastPage` mobile).
    pub position: i64,
    /// Terakhir dilihat, unix epoch detik (ala `History.lastViewed`).
    pub updated_at: i64,
}
