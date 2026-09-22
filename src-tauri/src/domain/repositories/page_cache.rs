//! PageCache port — simpan/cek/hapus halaman terunduh (8.1/8.2).
//! Implementasi konkret: `ImageCache` (data layer).

use std::sync::Arc;

use crate::core::AppError;

pub trait PageCache: Send + Sync {
    /// True bila halaman sudah ada di cache (resume skip).
    fn has(&self, source_id: &str, content_id: &str, page: u32) -> bool;
    /// Tulis bytes halaman (page = indeks 0-based).
    fn put(
        &self,
        source_id: &str,
        content_id: &str,
        page: u32,
        bytes: &[u8],
    ) -> Result<(), AppError>;
    /// Hapus seluruh halaman satu konten (hapus unduhan).
    fn remove_content(&self, source_id: &str, content_id: &str) -> Result<(), AppError>;
}

impl<R: PageCache + ?Sized> PageCache for Arc<R> {
    fn has(&self, source_id: &str, content_id: &str, page: u32) -> bool {
        (**self).has(source_id, content_id, page)
    }

    fn put(
        &self,
        source_id: &str,
        content_id: &str,
        page: u32,
        bytes: &[u8],
    ) -> Result<(), AppError> {
        (**self).put(source_id, content_id, page, bytes)
    }

    fn remove_content(&self, source_id: &str, content_id: &str) -> Result<(), AppError> {
        (**self).remove_content(source_id, content_id)
    }
}
