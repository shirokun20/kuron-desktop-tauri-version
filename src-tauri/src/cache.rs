//! ImageCache — ganti cached_network_image/extended_image (Fase 2).
//! Layout: `$root/$sourceId/$contentId/page_$n.jpg`, delegasi ke `FileCacheDs`.

use std::path::Path;

use crate::{core::AppError, data::datasources::local::FileCacheDs};

pub struct ImageCache {
    inner: FileCacheDs,
}

impl ImageCache {
    pub fn new(root: &Path, max_bytes: u64) -> Result<Self, AppError> {
        Ok(Self {
            inner: FileCacheDs::new(root, max_bytes)?,
        })
    }

    fn key(source_id: &str, content_id: &str, page: u32) -> String {
        format!("{source_id}/{content_id}/page_{page}.jpg")
    }

    pub fn cache_page(
        &self,
        source_id: &str,
        content_id: &str,
        page: u32,
        bytes: &[u8],
    ) -> Result<(), AppError> {
        self.inner
            .put(&Self::key(source_id, content_id, page), bytes)
    }

    pub fn page(
        &self,
        source_id: &str,
        content_id: &str,
        page: u32,
    ) -> Result<Option<Vec<u8>>, AppError> {
        self.inner.get(&Self::key(source_id, content_id, page))
    }

    pub fn has_page(&self, source_id: &str, content_id: &str, page: u32) -> bool {
        self.page(source_id, content_id, page)
            .map(|o| o.is_some())
            .unwrap_or(false)
    }

    /// Hapus seluruh halaman satu konten (`$root/$sourceId/$contentId/`).
    pub fn remove_content(&self, source_id: &str, content_id: &str) -> Result<(), AppError> {
        for part in [source_id, content_id] {
            if part.is_empty() || part.contains("..") || part.contains('/') || part.contains('\\') {
                return Err(AppError::Validation(format!("path komponen buruk: {part}")));
            }
        }
        let dir = self.inner.root().join(source_id).join(content_id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}

impl crate::domain::repositories::PageCache for ImageCache {
    fn has(&self, source_id: &str, content_id: &str, page: u32) -> bool {
        self.has_page(source_id, content_id, page)
    }

    fn put(
        &self,
        source_id: &str,
        content_id: &str,
        page: u32,
        bytes: &[u8],
    ) -> Result<(), AppError> {
        self.cache_page(source_id, content_id, page, bytes)
    }

    fn remove_content(&self, source_id: &str, content_id: &str) -> Result<(), AppError> {
        ImageCache::remove_content(self, source_id, content_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_roundtrip_layout() {
        let dir = std::env::temp_dir().join(format!("kuron-test-img-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = ImageCache::new(&dir, 1024 * 1024).unwrap();
        assert!(cache.page("nhentai", "m1", 1).unwrap().is_none());
        cache.cache_page("nhentai", "m1", 1, b"jpeg-bytes").unwrap();
        assert_eq!(
            cache.page("nhentai", "m1", 1).unwrap().unwrap(),
            b"jpeg-bytes"
        );
        assert!(dir.join("nhentai/m1/page_1.jpg").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
