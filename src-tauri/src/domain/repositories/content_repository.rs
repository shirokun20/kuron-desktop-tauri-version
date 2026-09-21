//! ContentRepository trait — port `repositories/content_repository.dart`.
//! Async via `async-trait` agar tetap object-safe di `Arc<dyn>` (Fase 1).

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    core::AppError,
    domain::{Chapter, Comment, Content, PageImageResult, SearchFilter},
};

#[async_trait]
pub trait HomeFeedRepository: Send + Sync {
    async fn home_feed(&self, page: u32) -> Result<Vec<Content>, AppError>;
}

#[async_trait]
pub trait ContentRepository: HomeFeedRepository {
    async fn search(&self, filter: SearchFilter) -> Result<Vec<Content>, AppError>;
    async fn get_detail(&self, content_id: &str) -> Result<Content, AppError>;
    async fn get_chapters(&self, content_id: &str) -> Result<Vec<Chapter>, AppError>;
    async fn get_page_images(&self, chapter_id: &str) -> Result<Vec<PageImageResult>, AppError>;
    /// Galeri terkait (ala `GetRelatedContentUseCase`; kosong = tak didukung).
    async fn get_related_content(&self, content_id: &str) -> Result<Vec<Content>, AppError>;
    /// Komentar galeri (ala `GetCommentsUseCase`; kosong = tak didukung).
    async fn get_comments(&self, content_id: &str) -> Result<Vec<Comment>, AppError>;
}

/// Blanket impl agar `Arc<dyn ContentRepository>` (isi AppState)
/// bisa dipakai langsung oleh use case generik.
#[async_trait]
impl<R: HomeFeedRepository + ?Sized> HomeFeedRepository for Arc<R> {
    async fn home_feed(&self, page: u32) -> Result<Vec<Content>, AppError> {
        (**self).home_feed(page).await
    }
}

#[async_trait]
impl<R: ContentRepository + ?Sized> ContentRepository for Arc<R> {
    async fn search(&self, filter: SearchFilter) -> Result<Vec<Content>, AppError> {
        (**self).search(filter).await
    }

    async fn get_detail(&self, content_id: &str) -> Result<Content, AppError> {
        (**self).get_detail(content_id).await
    }

    async fn get_chapters(&self, content_id: &str) -> Result<Vec<Chapter>, AppError> {
        (**self).get_chapters(content_id).await
    }

    async fn get_page_images(&self, chapter_id: &str) -> Result<Vec<PageImageResult>, AppError> {
        (**self).get_page_images(chapter_id).await
    }

    async fn get_related_content(&self, content_id: &str) -> Result<Vec<Content>, AppError> {
        (**self).get_related_content(content_id).await
    }

    async fn get_comments(&self, content_id: &str) -> Result<Vec<Comment>, AppError> {
        (**self).get_comments(content_id).await
    }
}
