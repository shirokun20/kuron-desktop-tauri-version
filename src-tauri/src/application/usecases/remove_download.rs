//! RemoveDownloadUseCase — hapus baris + cache unduhan (kontrol data penuh).

use std::sync::Arc;

use crate::{application::DownloadManager, core::AppError};

pub struct RemoveDownloadUseCase {
    manager: Arc<DownloadManager>,
}

impl RemoveDownloadUseCase {
    pub fn new(manager: Arc<DownloadManager>) -> Self {
        Self { manager }
    }

    pub async fn execute(&self, chapter_id: &str) -> Result<bool, AppError> {
        self.manager.remove(chapter_id).await
    }
}
