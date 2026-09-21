//! ClearLibraryUseCase — reset data library (subset `clearAllData` mobile:
//! riwayat + favorit + snapshot; unduhan milik Fase 6).

use crate::{core::AppError, domain::repositories::LibraryRepository};

pub struct ClearLibraryUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> ClearLibraryUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<(), AppError> {
        self.repo.clear_library().await
    }
}
