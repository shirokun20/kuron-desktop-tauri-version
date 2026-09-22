//! LibraryRepository trait — subset `UserDataRepository` mobile
//! (riwayat + favorit + reset, 8.3). Koleksi favorit, unduhan, preferensi,
//! dan sinkron offline milik Fase 6/7. Async via `async-trait` agar
//! object-safe di `Arc<dyn>` (pola `ContentRepository`).

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    core::AppError,
    domain::{Content, HistoryItem},
};

#[async_trait]
pub trait LibraryRepository: Send + Sync {
    /// Simpan/upsert posisi baca + snapshot display
    /// (ala `UserDataRepository.saveHistory` yang membawa `History` penuh).
    async fn record_history(&self, content: &Content, position: i64) -> Result<(), AppError>;
    /// Riwayat terbaru dulu, siap-display (ala `getHistory`).
    async fn list_history(&self, limit: i64) -> Result<Vec<HistoryItem>, AppError>;
    /// Buang satu entri riwayat (ala `removeFromHistory`).
    async fn remove_history(&self, content_id: &str) -> Result<(), AppError>;
    /// Buang seluruh riwayat (ala `clearHistory`).
    async fn clear_history(&self) -> Result<(), AppError>;
    /// Tandai favorit + simpan snapshot konten untuk daftar
    /// (ala `addToFavorites`/`removeFromFavorites` mobile yang menyimpan
    /// `title`/`coverUrl`/`sourceId` saat favorit ditandai).
    async fn set_favorite(&self, content: &Content, fav: bool) -> Result<(), AppError>;
    /// Daftar favorit terbaru dulu (ala `getFavorites`).
    async fn list_favorites(&self) -> Result<Vec<Content>, AppError>;
    /// Reset data library: riwayat + favorit + snapshot konten.
    /// Unduhan/cache terjemahan milik Fase 6/reader — tidak ikut.
    async fn clear_library(&self) -> Result<(), AppError>;
}

/// Blanket impl agar `Arc<dyn LibraryRepository>` (isi AppState)
/// bisa dipakai langsung oleh use case generik.
#[async_trait]
impl<R: LibraryRepository + ?Sized> LibraryRepository for Arc<R> {
    async fn record_history(&self, content: &Content, position: i64) -> Result<(), AppError> {
        (**self).record_history(content, position).await
    }

    async fn list_history(&self, limit: i64) -> Result<Vec<HistoryItem>, AppError> {
        (**self).list_history(limit).await
    }

    async fn remove_history(&self, content_id: &str) -> Result<(), AppError> {
        (**self).remove_history(content_id).await
    }

    async fn clear_history(&self) -> Result<(), AppError> {
        (**self).clear_history().await
    }

    async fn set_favorite(&self, content: &Content, fav: bool) -> Result<(), AppError> {
        (**self).set_favorite(content, fav).await
    }

    async fn list_favorites(&self) -> Result<Vec<Content>, AppError> {
        (**self).list_favorites().await
    }

    async fn clear_library(&self) -> Result<(), AppError> {
        (**self).clear_library().await
    }
}
