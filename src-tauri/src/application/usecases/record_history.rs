//! RecordHistoryUseCase — simpan posisi baca (ala `saveHistory` mobile).

use crate::{
    core::AppError,
    domain::repositories::LibraryRepository,
};

pub struct RecordHistoryUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> RecordHistoryUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content_id: &str, position: i64) -> Result<(), AppError> {
        self.repo.record_history(content_id, position).await
    }
}
