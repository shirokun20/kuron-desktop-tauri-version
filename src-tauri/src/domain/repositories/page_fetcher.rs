//! PageFetcher port — ambil bytes satu halaman (download manager 8.1).
//! Dipisah dari `ContentRepository` agar manager bisa diuji tanpa HTTP live.

use std::sync::Arc;

use async_trait::async_trait;

use crate::core::AppError;

#[async_trait]
pub trait PageFetcher: Send + Sync {
    /// GET bytes gambar; `referer` opsional (hotlink guard).
    async fn fetch(
        &self,
        url: &str,
        source_id: &str,
        referer: Option<&str>,
    ) -> Result<Vec<u8>, AppError>;
}

#[async_trait]
impl<R: PageFetcher + ?Sized> PageFetcher for Arc<R> {
    async fn fetch(
        &self,
        url: &str,
        source_id: &str,
        referer: Option<&str>,
    ) -> Result<Vec<u8>, AppError> {
        (**self).fetch(url, source_id, referer).await
    }
}
