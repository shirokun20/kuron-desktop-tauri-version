//! MockContentRepository — fixture in-memory (anti-big-bang, spec §20).
//! Diganti `ContentRepositoryImpl` (remote+local) di Fase 2.

use async_trait::async_trait;

use crate::{
    core::AppError,
    domain::{
        repositories::{ContentRepository, HomeFeedRepository},
        Chapter, Content, PageImageResult, SearchFilter,
    },
};

pub struct MockContentRepository;

#[async_trait]
impl HomeFeedRepository for MockContentRepository {
    async fn home_feed(&self) -> Result<Vec<Content>, AppError> {
        Ok(Content::mock_feed())
    }
}

#[async_trait]
impl ContentRepository for MockContentRepository {
    async fn search(&self, _filter: SearchFilter) -> Result<Vec<Content>, AppError> {
        Ok(Content::mock_feed())
    }

    async fn get_detail(&self, content_id: &str) -> Result<Content, AppError> {
        Ok(Content {
            id: content_id.to_string(),
            title: format!("Mock detail {content_id}"),
            cover_url: String::new(),
            source_id: "nhentai".to_string(),
            upload_date: None,
            is_favorite: false,
        })
    }

    async fn get_chapters(&self, _content_id: &str) -> Result<Vec<Chapter>, AppError> {
        Ok(vec![])
    }

    async fn get_page_images(
        &self,
        _chapter_id: &str,
    ) -> Result<Vec<PageImageResult>, AppError> {
        Ok(vec![])
    }
}
