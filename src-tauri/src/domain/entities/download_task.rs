//! DownloadTask — port `entities/download_task.dart` + `download_status.dart`.
//! `source_id` ditambahkan desktop: kunci layout ImageCache + resume
//! setelah restart aplikasi (mobile resolve via repo aktif).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum DownloadState {
    Queued,
    Downloading { page: u32, total: u32 },
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DownloadTask {
    pub chapter_id: String,
    pub content_id: String,
    /// Sumber chapter (kunci cache `$sourceId/$contentId/page_N.jpg`).
    pub source_id: String,
    pub state: DownloadState,
}
