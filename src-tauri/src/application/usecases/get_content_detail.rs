//! GetContentDetailUseCase — metadata + cover satu konten.

use crate::{
    core::AppError,
    domain::{repositories::ContentRepository, Content},
};

pub struct GetContentDetailUseCase<R> {
    repo: R,
}

impl<R: ContentRepository> GetContentDetailUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content_id: &str) -> Result<Content, AppError> {
        self.repo.get_detail(content_id).await
    }
}
