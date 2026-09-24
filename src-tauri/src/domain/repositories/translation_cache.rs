//! TranslationCacheRepository trait — cache hasil terjemahan per halaman
//! (port `TranslationCacheRepository` mobile, 7.2).
//! Kunci: SHA256 16 hex (`TranslatePageUseCase::cache_key`).

use std::sync::Arc;

use async_trait::async_trait;

use crate::{core::AppError, domain::PageTranslation};

#[async_trait]
pub trait TranslationCacheRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<PageTranslation>, AppError>;
    async fn put(&self, key: &str, value: &PageTranslation) -> Result<(), AppError>;
}

/// Blanket impl agar `Arc<dyn TranslationCacheRepository>` bisa dipakai
/// langsung oleh use case generik.
#[async_trait]
impl<R: TranslationCacheRepository + ?Sized> TranslationCacheRepository for Arc<R> {
    async fn get(&self, key: &str) -> Result<Option<PageTranslation>, AppError> {
        (**self).get(key).await
    }

    async fn put(&self, key: &str, value: &PageTranslation) -> Result<(), AppError> {
        (**self).put(key, value).await
    }
}
