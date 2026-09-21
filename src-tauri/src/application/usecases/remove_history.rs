//! RemoveHistoryUseCase — buang satu entri (ala `removeFromHistory` mobile).

use crate::{core::AppError, domain::repositories::LibraryRepository};

pub struct RemoveHistoryUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> RemoveHistoryUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content_id: &str) -> Result<(), AppError> {
        self.repo.remove_history(content_id).await
    }
}
