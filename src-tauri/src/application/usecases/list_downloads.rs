//! ListDownloadsUseCase — daftar task unduhan (halaman offline 8.1/8.2).

use crate::{
    core::AppError,
    domain::{repositories::DownloadsRepository, DownloadTask},
};

pub struct ListDownloadsUseCase<R> {
    repo: R,
}

impl<R: DownloadsRepository> ListDownloadsUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<Vec<DownloadTask>, AppError> {
        self.repo.list().await
    }
}
