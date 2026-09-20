//! GetChaptersUseCase — daftar chapter satu konten.

use crate::{
    core::AppError,
    domain::{repositories::ContentRepository, Chapter},
};

pub struct GetChaptersUseCase<R> {
    repo: R,
}

impl<R: ContentRepository> GetChaptersUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content_id: &str) -> Result<Vec<Chapter>, AppError> {
        self.repo.get_chapters(content_id).await
    }
}
