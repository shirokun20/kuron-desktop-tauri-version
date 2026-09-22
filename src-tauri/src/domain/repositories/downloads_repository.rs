//! DownloadsRepository trait — unduhan chapter (8.1, spec `offline-download`).
//! Baris SQLite `downloads` + state_json `DownloadState`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{core::AppError, domain::DownloadTask};

#[async_trait]
pub trait DownloadsRepository: Send + Sync {
    /// Simpan/upsert satu task (chapter_id sebagai kunci).
    async fn save(&self, task: &DownloadTask) -> Result<(), AppError>;
    /// Ambil satu task; None bila tak ada.
    async fn get(&self, chapter_id: &str) -> Result<Option<DownloadTask>, AppError>;
    /// Semua task (untuk daftar offline + resume setelah restart).
    async fn list(&self) -> Result<Vec<DownloadTask>, AppError>;
    /// Hapus satu task (cache file diurus terpisah oleh use case).
    async fn delete(&self, chapter_id: &str) -> Result<bool, AppError>;
}

/// Blanket impl agar `Arc<dyn DownloadsRepository>` (isi AppState)
/// bisa dipakai langsung oleh use case generik.
#[async_trait]
impl<R: DownloadsRepository + ?Sized> DownloadsRepository for Arc<R> {
    async fn save(&self, task: &DownloadTask) -> Result<(), AppError> {
        (**self).save(task).await
    }

    async fn get(&self, chapter_id: &str) -> Result<Option<DownloadTask>, AppError> {
        (**self).get(chapter_id).await
    }

    async fn list(&self) -> Result<Vec<DownloadTask>, AppError> {
        (**self).list().await
    }

    async fn delete(&self, chapter_id: &str) -> Result<bool, AppError> {
        (**self).delete(chapter_id).await
    }
}

/// Blanket impl `&R` untuk test usecase.
#[async_trait]
impl<'a, R: DownloadsRepository + ?Sized> DownloadsRepository for &'a R {
    async fn save(&self, task: &DownloadTask) -> Result<(), AppError> {
        (**self).save(task).await
    }

    async fn get(&self, chapter_id: &str) -> Result<Option<DownloadTask>, AppError> {
        (**self).get(chapter_id).await
    }

    async fn list(&self) -> Result<Vec<DownloadTask>, AppError> {
        (**self).list().await
    }

    async fn delete(&self, chapter_id: &str) -> Result<bool, AppError> {
        (**self).delete(chapter_id).await
    }
}
