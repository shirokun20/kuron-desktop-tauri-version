//! TranslationCacheRepositoryImpl — cache JSON `PageTranslation` di tabel
//! `translation_cache` SQLite (7.2). Kunci dari use case (SHA256 16 hex).

use std::path::Path;

use async_trait::async_trait;

use crate::{
    core::AppError,
    data::datasources::local::SqliteDs,
    domain::{repositories::TranslationCacheRepository, PageTranslation},
};

pub struct TranslationCacheRepositoryImpl {
    db: SqliteDs,
}

impl TranslationCacheRepositoryImpl {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            db: SqliteDs::open(path)?,
        })
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        Ok(Self {
            db: SqliteDs::open_in_memory()?,
        })
    }
}

#[async_trait]
impl TranslationCacheRepository for TranslationCacheRepositoryImpl {
    async fn get(&self, key: &str) -> Result<Option<PageTranslation>, AppError> {
        let Some(raw) = self.db.get_translation(key)? else {
            return Ok(None);
        };
        let value: PageTranslation = serde_json::from_slice(&raw)
            .map_err(|e| AppError::Storage(format!("cache terjemahan rusak: {e}")))?;
        Ok(Some(value))
    }

    async fn put(&self, key: &str, value: &PageTranslation) -> Result<(), AppError> {
        let raw = serde_json::to_vec(value)
            .map_err(|e| AppError::Storage(format!("encode cache terjemahan: {e}")))?;
        self.db.put_translation(key, &raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TranslatedBubble;

    fn sample() -> PageTranslation {
        PageTranslation {
            page_index: 3,
            bubbles: vec![TranslatedBubble {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
                original: "おはよう".into(),
                reading: "ohayou".into(),
                translated: "selamat pagi".into(),
                shape: None,
                needs_white_patch: false,
            }],
            detected_lang: "ja".into(),
            used_fallback: false,
        }
    }

    #[test]
    fn put_lalu_get_bulat() {
        let repo = TranslationCacheRepositoryImpl::open_in_memory().unwrap();
        assert!(tauri::async_runtime::block_on(repo.get("k1"))
            .unwrap()
            .is_none());
        tauri::async_runtime::block_on(repo.put("k1", &sample())).unwrap();
        let back = tauri::async_runtime::block_on(repo.get("k1"))
            .unwrap()
            .expect("ada");
        assert_eq!(back.page_index, 3);
        assert_eq!(back.bubbles[0].translated, "selamat pagi");
        // Upsert menimpa.
        let mut v2 = sample();
        v2.detected_lang = "ko".into();
        tauri::async_runtime::block_on(repo.put("k1", &v2)).unwrap();
        assert_eq!(
            tauri::async_runtime::block_on(repo.get("k1"))
                .unwrap()
                .unwrap()
                .detected_lang,
            "ko"
        );
    }
}
