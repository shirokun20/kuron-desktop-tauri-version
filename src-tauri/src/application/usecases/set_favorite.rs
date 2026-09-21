//! SetFavoriteUseCase — tandai/hapus favorit + snapshot konten
//! (ala `addToFavorites`/`removeFromFavorites` mobile).

use crate::{
    core::AppError,
    domain::{repositories::LibraryRepository, Content},
};

pub struct SetFavoriteUseCase<R> {
    repo: R,
}

impl<R: LibraryRepository> SetFavoriteUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, content: &Content, fav: bool) -> Result<(), AppError> {
        self.repo.set_favorite(content, fav).await
    }
}
