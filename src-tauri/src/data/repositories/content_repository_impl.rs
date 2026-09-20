//! MockContentRepository — fixture in-memory (anti-big-bang, spec §20).
//! `ContentRepositoryImpl` di bawah: remote-first (adapter), SQLite nyusul
//! untuk cache/favorit/riwayat saat wiring AppState Fase 6.

use async_trait::async_trait;

use crate::{
    core::AppError,
    data::{
        datasources::remote::{
            GenericRestAdapter, GenericScraperAdapter, NhentaiApiAdapter, SourceConfig,
        },
        models::ContentModel,
    },
    domain::{
        repositories::{ContentRepository, HomeFeedRepository},
        Chapter, Content, PageImageResult, SearchFilter,
    },
};

pub struct MockContentRepository;

#[async_trait]
impl HomeFeedRepository for MockContentRepository {
    async fn home_feed(&self, page: u32) -> Result<Vec<Content>, AppError> {
        if page > 1 {
            return Ok(vec![]);
        }
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
            page_count: None,
            language: None,
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

/// Real impl: remote-first via adapter per sumber aktif.
/// `config.source_id == "mangadex"` → REST API; lainnya → scraper HTML.
pub struct ContentRepositoryImpl {
    scraper: GenericScraperAdapter,
    rest: GenericRestAdapter,
    config: SourceConfig,
    nhentai: Option<NhentaiApiAdapter>,
}

impl ContentRepositoryImpl {
    pub fn new(
        scraper: GenericScraperAdapter,
        rest: GenericRestAdapter,
        config: SourceConfig,
    ) -> Self {
        Self {
            scraper,
            rest,
            config,
            nhentai: None,
        }
    }

    pub fn with_nhentai(mut self, adapter: NhentaiApiAdapter) -> Self {
        self.nhentai = Some(adapter);
        self
    }

    fn is_mangadex(&self) -> bool {
        self.config.source_id == "mangadex"
    }

    fn is_nhentai(&self) -> bool {
        self.config.source_id == "nhentai" && self.nhentai.is_some()
    }

    fn nh(&self) -> Result<&NhentaiApiAdapter, AppError> {
        self.nhentai.as_ref().ok_or_else(|| {
            AppError::Internal("nhentai adapter belum dipasang".to_string())
        })
    }
}

#[async_trait]
impl HomeFeedRepository for ContentRepositoryImpl {
    async fn home_feed(&self, page: u32) -> Result<Vec<Content>, AppError> {
        let page = page.max(1);
        let models = if self.is_nhentai() {
            self.nh()?.search("", page).await?
        } else if self.is_mangadex() {
            self.rest.search_mangadex("", page).await?
        } else {
            let url = self.config.home_url_page(page);
            self.scraper.fetch_list(&url, &self.config).await?
        };
        if models.is_empty() {
            tracing::warn!(source = %self.config.source_id, "home_feed kosong (blokir/CF? markup berubah?)");
        }
        Ok(models.into_iter().map(Content::from).collect())
    }
}

#[async_trait]
impl ContentRepository for ContentRepositoryImpl {
    async fn search(&self, filter: SearchFilter) -> Result<Vec<Content>, AppError> {
        let page = filter.page.max(1);
        let models = if self.is_nhentai() {
            self.nh()?.search(&filter.query, page).await?
        } else if self.is_mangadex() {
            self.rest.search_mangadex(&filter.query, page).await?
        } else {
            let url = self.config.list_url(&filter.query, page);
            self.scraper.fetch_list(&url, &self.config).await?
        };
        Ok(models.into_iter().map(Content::from).collect())
    }

    async fn get_detail(&self, content_id: &str) -> Result<Content, AppError> {
        let model: ContentModel = if self.is_nhentai() {
            self.nh()?.detail(content_id).await?
        } else if self.is_mangadex() {
            self.rest.get_mangadex_detail(content_id).await?
        } else {
            let url = self.config.detail_url(content_id);
            self.scraper.fetch_detail(&url, &self.config).await?
        };
        Ok(Content::from(model))
    }

    async fn get_chapters(&self, content_id: &str) -> Result<Vec<Chapter>, AppError> {
        if self.is_nhentai() {
            return Ok(vec![Chapter {
                id: format!("{content_id}-1"),
                content_id: content_id.to_string(),
                title: "Baca".to_string(),
                order: 1,
                is_external: false,
                external_url: Some(format!("https://nhentai.net/g/{content_id}/")),
            }]);
        }
        if self.is_mangadex() {
            // MangaDex chapter feed (aggregate) — nyusul: butuh ChapterModel + feed API.
            return Err(AppError::Internal(
                "mangadex chapters: endpoint aggregate nyusul".to_string(),
            ));
        }
        let url = self.config.detail_url(content_id);
        self.scraper.fetch_chapters(&url, content_id, &self.config).await
    }

    async fn get_page_images(
        &self,
        chapter_id: &str,
    ) -> Result<Vec<PageImageResult>, AppError> {
        if self.is_nhentai() {
            let gallery = NhentaiApiAdapter::gallery_of_chapter(chapter_id);
            let urls = self.nh()?.pages(gallery).await?;
            return Ok(urls.into_iter().map(PageImageResult::Remote).collect());
        }
        if self.is_mangadex() {
            return Err(AppError::Internal(
                "mangadex pages: endpoint at-home nyusul".to_string(),
            ));
        }
        // Frontend kirim `chapter.external_url` (URL penuh) bila ada.
        let urls = self
            .scraper
            .fetch_page_images(&self.config.detail_url(chapter_id), &self.config)
            .await?;
        Ok(urls.into_iter().map(PageImageResult::Remote).collect())
    }
}
