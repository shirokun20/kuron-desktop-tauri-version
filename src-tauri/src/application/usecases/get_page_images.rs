//! GetPageImagesUseCase — URL gambar per halaman satu chapter.

use crate::{
    core::AppError,
    domain::{repositories::ContentRepository, PageImageResult},
};

pub struct GetPageImagesUseCase<R> {
    repo: R,
}

impl<R: ContentRepository> GetPageImagesUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, chapter_id: &str) -> Result<Vec<PageImageResult>, AppError> {
        self.repo.get_page_images(chapter_id).await
    }
}
