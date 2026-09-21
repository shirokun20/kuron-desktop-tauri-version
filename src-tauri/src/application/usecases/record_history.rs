//! RecordHistoryUseCase — simpan posisi baca (ala `saveHistory` mobile).

use crate::{
    core::AppError,
    domain::{repositories::LibraryRepository, Content},
};

pub struct RecordHistoryUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> RecordHistoryUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content: &Content, position: i64) -> Result<(), AppError> {
        self.repo.record_history(content, position).await
    }
}
