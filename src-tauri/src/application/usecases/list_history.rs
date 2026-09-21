//! ListHistoryUseCase — riwayat terbaru dulu (ala `getHistory` mobile).

use crate::{
    core::AppError,
    domain::{repositories::LibraryRepository, HistoryEntry},
};

pub struct ListHistoryUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> ListHistoryUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, limit: i64) -> Result<Vec<HistoryEntry>, AppError> {
        self.repo.list_history(limit).await
    }
}
