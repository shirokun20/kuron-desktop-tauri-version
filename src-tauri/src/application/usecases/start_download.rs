//! StartDownloadUseCase — ambil URL halaman lalu serahkan ke DownloadManager.

use std::sync::Arc;

use crate::{
    application::DownloadManager,
    core::AppError,
    domain::{repositories::ContentRepository, DownloadTask},
};

pub struct StartDownloadUseCase<C> {
    repo: C,
    manager: Arc<DownloadManager>,
}

impl<C: ContentRepository> StartDownloadUseCase<C> {
    pub fn new(repo: C, manager: Arc<DownloadManager>) -> Self {
        Self { repo, manager }
    }

    /// `total_hint` = page_count badge (dihormati bila > 0; URL menang bila beda).
    pub async fn execute(
        &self,
        chapter_id: &str,
        content_id: &str,
        source_id: &str,
        total_hint: Option<u32>,
    ) -> Result<DownloadTask, AppError> {
        let pages = self.repo.get_page_images(chapter_id).await?;
        let url_count = pages
            .iter()
            .filter(|p| matches!(p, crate::domain::PageImageResult::Remote(_)))
            .count() as u32;
        let total = total_hint.filter(|t| *t > 0).unwrap_or(url_count);
        self.manager
            .start(chapter_id, content_id, source_id, pages, total)
            .await
    }
}
