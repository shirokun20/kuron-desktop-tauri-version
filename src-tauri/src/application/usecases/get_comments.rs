//! GetCommentsUseCase — komentar galeri (port mobile 1:1).

use crate::{
    core::AppError,
    domain::{repositories::ContentRepository, Comment},
};

pub struct GetCommentsUseCase<R> {
    repo: R,
}

impl<R: ContentRepository> GetCommentsUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content_id: &str) -> Result<Vec<Comment>, AppError> {
        self.repo.get_comments(content_id).await
    }
}
