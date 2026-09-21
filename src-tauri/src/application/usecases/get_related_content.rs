//! GetRelatedContentUseCase — galeri terkait (port mobile 1:1).

use crate::{
    core::AppError,
    domain::{repositories::ContentRepository, Content},
};

pub struct GetRelatedContentUseCase<R> {
    repo: R,
}

impl<R: ContentRepository> GetRelatedContentUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content_id: &str) -> Result<Vec<Content>, AppError> {
        self.repo.get_related_content(content_id).await
    }
}
