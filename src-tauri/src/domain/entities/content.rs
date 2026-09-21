//! Content entity — port `entities/content.dart`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::tag::Tag;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Content {
    pub id: String,
    pub title: String,
    pub cover_url: String,
    pub source_id: String,
    pub upload_date: Option<String>,
    pub is_favorite: bool,
    /// Jumlah halaman (badge kartu); None = tak diketahui sumber ini.
    #[serde(default)]
    pub page_count: Option<u32>,
    /// Kode bahasa ISO (en/ja/zh/id); None = tak diketahui.
    #[serde(default)]
    pub language: Option<String>,
    /// Tag metadata (nhentai `tags[]`); kosong = sumber tak sediakan.
    #[serde(default)]
    pub tags: Vec<Tag>,
}

impl Content {
    /// Fixture untuk mock commands (strategi anti-big-bang, spec §20).
    pub fn mock_feed() -> Vec<Self> {
        (1..=8)
            .map(|i| Self {
                id: format!("mock-{i}"),
                title: format!("Mock Gallery {i} — splash ke main OK"),
                cover_url: String::new(),
                source_id: "nhentai".to_string(),
                upload_date: None,
                is_favorite: false,
                page_count: None,
                language: None,
                tags: Vec::new(),
            })
            .collect()
    }
}
