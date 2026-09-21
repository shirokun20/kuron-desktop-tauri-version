//! MockContentRepository — fixture in-memory (anti-big-bang, spec §20).
//! `ContentRepositoryImpl` di bawah: remote-first (adapter), SQLite nyusul
//! untuk cache/favorit/riwayat saat wiring AppState Fase 6.

use async_trait::async_trait;

use crate::{
    core::AppError,
    data::{
        datasources::remote::{
            CursorState, GenericRestAdapter, GenericScraperAdapter, NhentaiApiAdapter,
            PaginationCursors, SourceConfig,
        },
        models::ContentModel,
    },
    domain::{
        repositories::{ContentRepository, HomeFeedRepository},
        Chapter, Comment, Content, PageImageResult, SearchFilter,
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
            tags: Vec::new(),
            available_languages: Vec::new(),
            description: None,
            rating: None,
            favorites: None,
        })
    }

    async fn get_chapters(&self, _content_id: &str, _language: Option<&str>, _offset: Option<u32>) -> Result<Vec<Chapter>, AppError> {
        Ok(vec![])
    }

    async fn get_page_images(&self, _chapter_id: &str) -> Result<Vec<PageImageResult>, AppError> {
        Ok(vec![])
    }

    async fn get_related_content(&self, _content_id: &str) -> Result<Vec<Content>, AppError> {
        Ok(vec![])
    }

    async fn get_comments(&self, _content_id: &str) -> Result<Vec<Comment>, AppError> {
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
    cursors: PaginationCursors,
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
            cursors: PaginationCursors::default(),
        }
    }

    pub fn with_pagination(mut self, cursors: PaginationCursors) -> Self {
        self.cursors = cursors;
        self
    }

    pub fn with_nhentai(mut self, adapter: NhentaiApiAdapter) -> Self {
        self.nhentai = Some(adapter);
        self
    }

    /// Fetch scraper + putar cursor token (mobile `_fetchListPage`):
    /// halaman N+1 dari link `next` bila ada, template bila tak ada,
    /// kosong tanpa fetch bila halaman berikut sudah terbukti habis.
    async fn fetch_scraper_home(&self, page: u32) -> Result<Vec<ContentModel>, AppError> {
        let page = page.max(1);
        let key = PaginationCursors::home_key(&self.config.source_id, page);
        let next_key = PaginationCursors::home_key(&self.config.source_id, page + 1);
        self.fetch_scraper_cursored(&key, &next_key, || self.config.home_url_page(page))
            .await
    }

    async fn fetch_scraper_search(
        &self,
        query: &str,
        page: u32,
    ) -> Result<Vec<ContentModel>, AppError> {
        let page = page.max(1);
        let key = PaginationCursors::search_key(&self.config.source_id, query, page);
        let next_key = PaginationCursors::search_key(&self.config.source_id, query, page + 1);
        // Konvensi `raw:` mobile: payload param mentah ganti query template.
        let template = if query.starts_with("raw:") {
            self.config.search_url_raw(query, page)
        } else {
            self.config.list_url(query, page)
        };
        self.fetch_scraper_cursored(&key, &next_key, || template)
            .await
    }

    async fn fetch_scraper_cursored(
        &self,
        key: &str,
        next_key: &str,
        template_url: impl FnOnce() -> String,
    ) -> Result<Vec<ContentModel>, AppError> {
        let url = match self.cursors.get(key) {
            Some(CursorState::Exhausted) => return Ok(vec![]),
            Some(CursorState::Next(u)) => u,
            None => template_url(),
        };
        let (models, next) = self
            .scraper
            .fetch_list_with_next(&url, &self.config)
            .await?;
        match next {
            Some(u) => self.cursors.put_next(next_key, u),
            None => self.cursors.mark_exhausted(next_key),
        }
        Ok(models)
    }

    fn is_mangadex(&self) -> bool {
        self.config.source_id == "mangadex"
    }

    fn is_ehentai(&self) -> bool {
        self.config.source_id == "ehentai"
    }

    fn is_nhentai(&self) -> bool {
        self.config.source_id == "nhentai" && self.nhentai.is_some()
    }

    fn nh(&self) -> Result<&NhentaiApiAdapter, AppError> {
        self.nhentai
            .as_ref()
            .ok_or_else(|| AppError::Internal("nhentai adapter belum dipasang".to_string()))
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
            self.fetch_scraper_home(page).await?
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
            self.fetch_scraper_search(&filter.query, page).await?
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

    async fn get_chapters(&self, content_id: &str, language: Option<&str>, offset: Option<u32>) -> Result<Vec<Chapter>, AppError> {
        if self.is_nhentai() {
            return Ok(vec![Chapter {
                id: format!("{content_id}-1"),
                content_id: content_id.to_string(),
                title: "Baca".to_string(),
                order: 1,
                is_external: false,
                external_url: Some(format!("https://nhentai.net/g/{content_id}/")),
                language: None,
            }]);
        }
        if self.is_mangadex() {
            let chapters = self.rest.chapters_mangadex(content_id, language, offset).await?;
            return Ok(chapters);
        }
        let url = self.config.detail_url(content_id);
        self.scraper
            .fetch_chapters(&url, content_id, &self.config)
            .await
    }

    async fn get_page_images(&self, chapter_id: &str) -> Result<Vec<PageImageResult>, AppError> {
        if self.is_nhentai() {
            let gallery = NhentaiApiAdapter::gallery_of_chapter(chapter_id);
            let urls = self.nh()?.pages(gallery).await?;
            return Ok(urls.into_iter().map(PageImageResult::Remote).collect());
        }
        if self.is_mangadex() {
            if chapter_id.starts_with("http://") || chapter_id.starts_with("https://") {
                return Err(AppError::Validation(
                    "chapter eksternal tak bisa dibuka di reader".to_string(),
                ));
            }
            let urls = self.rest.pages_mangadex_at_home(chapter_id).await?;
            return Ok(urls.into_iter().map(PageImageResult::Remote).collect());
        }
        if self.is_ehentai() {
            // Reader E-Hentai lazy per `/s/` (40+ request/part) butuh Fase 5;
            // daftar Part kini benar, gambar nyusul arsitektur lazy mobile.
            return Err(AppError::Internal(
                "ehentai pages: reader lazy nyusul Fase 5".to_string(),
            ));
        }
        // Frontend kirim `chapter.external_url` (URL penuh) bila ada.
        let urls = self
            .scraper
            .fetch_page_images(&self.config.detail_url(chapter_id), &self.config)
            .await?;
        Ok(urls.into_iter().map(PageImageResult::Remote).collect())
    }

    async fn get_related_content(&self, content_id: &str) -> Result<Vec<Content>, AppError> {
        // nhentai: endpoint `related`; MD: batch rekomendasi (mobile
        // `_fetchRelatedBatch`); sumber lain kosong jujur.
        if self.is_nhentai() {
            let models = self.nh()?.related(content_id).await?;
            return Ok(models.into_iter().map(Content::from).collect());
        }
        if self.is_mangadex() {
            let models = self.rest.related_mangadex(content_id).await?;
            return Ok(models.into_iter().map(Content::from).collect());
        }
        Ok(vec![])
    }

    async fn get_comments(&self, content_id: &str) -> Result<Vec<Comment>, AppError> {
        // Hanya nhentai (`?include=comments` embedded); sumber lain kosong.
        if !self.is_nhentai() {
            return Ok(vec![]);
        }
        self.nh()?.comments(content_id).await
    }
}
