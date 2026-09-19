//! MockContentRepository — fixture in-memory (anti-big-bang, spec §20).
//! Diganti `ContentRepositoryImpl` (remote+local) di Fase 2.

use crate::{
    core::AppError,
    domain::{
        repositories::{ContentRepository, HomeFeedRepository},
        Chapter, Content, PageImageResult, SearchFilter,
    },
};

pub struct MockContentRepository;

impl HomeFeedRepository for MockContentRepository {
    fn home_feed(&self) -> Result<Vec<Content>, AppError> {
        Ok(Content::mock_feed())
    }
}

impl ContentRepository for MockContentRepository {
    fn search(&self, _filter: SearchFilter) -> Result<Vec<Content>, AppError> {
        Ok(Content::mock_feed())
    }

    fn get_detail(&self, content_id: &str) -> Result<Content, AppError> {
        Ok(Content {
            id: content_id.to_string(),
            title: format!("Mock detail {content_id}"),
            cover_url: String::new(),
            source_id: "nhentai".to_string(),
            upload_date: None,
            is_favorite: false,
        })
    }

    fn get_chapters(&self, _content_id: &str) -> Result<Vec<Chapter>, AppError> {
        Ok(vec![])
    }

    fn get_page_images(&self, _chapter_id: &str) -> Result<Vec<PageImageResult>, AppError> {
        Ok(vec![])
    }
}
