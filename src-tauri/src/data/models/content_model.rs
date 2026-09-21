//! ContentModel — serde + From Entity (port `fromEntity/toEntity/fromMap`).
//! `ts-rs` generate types nyusul ADR-004.

use serde::{Deserialize, Serialize};

use crate::domain::{Content, Tag};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentModel {
    pub id: String,
    pub title: String,
    pub cover_url: String,
    pub source_id: String,
    pub upload_date: Option<String>,
    #[serde(default)]
    pub page_count: Option<u32>,
    #[serde(default)]
    pub language: Option<String>,
    /// Tag metadata (detail nhentai); kosong bila sumber tak sediakan.
    #[serde(default)]
    pub tags: Vec<Tag>,
}

impl From<ContentModel> for Content {
    fn from(m: ContentModel) -> Self {
        Self {
            id: m.id,
            title: m.title,
            cover_url: m.cover_url,
            source_id: m.source_id,
            upload_date: m.upload_date,
            is_favorite: false,
            page_count: m.page_count,
            language: m.language,
            tags: m.tags,
        }
    }
}

impl From<Content> for ContentModel {
    fn from(e: Content) -> Self {
        Self {
            id: e.id,
            title: e.title,
            cover_url: e.cover_url,
            source_id: e.source_id,
            upload_date: e.upload_date,
            page_count: e.page_count,
            language: e.language,
            tags: e.tags,
        }
    }
}
