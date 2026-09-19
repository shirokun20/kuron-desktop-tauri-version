//! DownloadTask — port `entities/download_task.dart` + `download_status.dart`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadState {
    Queued,
    Downloading { page: u32, total: u32 },
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub chapter_id: String,
    pub content_id: String,
    pub state: DownloadState,
}
