//! SearchContentUseCase — cari lintas sumber (filter + pagination).

use crate::{
    core::AppError,
    domain::{repositories::ContentRepository, Content, SearchFilter},
};

pub struct SearchContentUseCase<R> {
    repo: R,
}

impl<R: ContentRepository> SearchContentUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, filter: SearchFilter) -> Result<Vec<Content>, AppError> {
        self.repo.search(filter).await
    }
}
