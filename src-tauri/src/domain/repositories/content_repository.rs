//! ContentRepository trait — port `repositories/content_repository.dart`.

use crate::{
    core::AppError,
    domain::{Chapter, Content, PageImageResult, SearchFilter},
};

pub trait HomeFeedRepository {
    fn home_feed(&self) -> Result<Vec<Content>, AppError>;
}

pub trait ContentRepository: HomeFeedRepository {
    fn search(&self, filter: SearchFilter) -> Result<Vec<Content>, AppError>;
    fn get_detail(&self, content_id: &str) -> Result<Content, AppError>;
    fn get_chapters(&self, content_id: &str) -> Result<Vec<Chapter>, AppError>;
    fn get_page_images(&self, chapter_id: &str) -> Result<Vec<PageImageResult>, AppError>;
}
