//! ListFavoritesUseCase — favorit + snapshot display (ala `getFavorites`).

use crate::{
    core::AppError,
    domain::{repositories::LibraryRepository, Content},
};

pub struct ListFavoritesUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> ListFavoritesUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<Vec<Content>, AppError> {
        self.repo.list_favorites().await
    }
}
