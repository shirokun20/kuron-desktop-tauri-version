//! ClearHistoryUseCase — buang seluruh riwayat (ala `clearHistory` mobile).

use crate::{core::AppError, domain::repositories::LibraryRepository};

pub struct ClearHistoryUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> ClearHistoryUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<(), AppError> {
        self.repo.clear_history().await
    }
}
