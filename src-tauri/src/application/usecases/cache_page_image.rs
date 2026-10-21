//! CachePageImageUseCase — hangatkan satu halaman reader ke disk (B).
//! Hit cache → `Cached(path)` tanpa network; miss → fetch via `PageFetcher`
//! (UA/cookie-jar backend) → `put` → `Cached`; gagal di titik mana pun →
//! `Remote(url)` agar WebView coba langsung (perilaku lama, tanpa error merah).
//! Satu halaman = satu `execute` agar FE bisa progresif per halaman.

use crate::{
    core::AppError,
    domain::{
        repositories::{PageCache, PageFetcher},
        PageImageResult,
    },
};

pub struct CachePageImageUseCase<F, C> {
    fetcher: F,
    cache: C,
}

impl<F: PageFetcher, C: PageCache> CachePageImageUseCase<F, C> {
    pub fn new(fetcher: F, cache: C) -> Self {
        Self { fetcher, cache }
    }

    pub async fn execute(
        &self,
        source_id: &str,
        content_id: &str,
        page: u32,
        url: &str,
    ) -> Result<PageImageResult, AppError> {
        if source_id.is_empty() || content_id.is_empty() {
            return Err(AppError::Validation("source/content kosong".to_string()));
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(AppError::Validation("url halaman buruk".to_string()));
        }
        // Validasi path dulu (gagal = Validation, bukan fallback).
        let path = self.cache.path(source_id, content_id, page)?;
        if self.cache.has(source_id, content_id, page) {
            return Ok(PageImageResult::Cached(path_string(&path)));
        }
        let bytes = match self.fetcher.fetch(url, source_id, None).await {
            Ok(b) if !b.is_empty() => b,
            _ => return Ok(PageImageResult::Remote(url.to_string())),
        };
        if self.cache.put(source_id, content_id, page, &bytes).is_err() {
            return Ok(PageImageResult::Remote(url.to_string()));
        }
        Ok(PageImageResult::Cached(path_string(&path)))
    }
}

fn path_string(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, path::PathBuf, sync::Mutex};

    struct ScriptedFetcher {
        payload: Vec<u8>,
        fail: bool,
        calls: Mutex<usize>,
    }

    #[async_trait::async_trait]
    impl PageFetcher for ScriptedFetcher {
        async fn fetch(
            &self,
            _url: &str,
            _source_id: &str,
            _referer: Option<&str>,
        ) -> Result<Vec<u8>, AppError> {
            *self.calls.lock().unwrap_or_else(|e| e.into_inner()) += 1;
            if self.fail {
                return Err(AppError::Network("HTTP 500".into()));
            }
            Ok(self.payload.clone())
        }
    }

    struct MemCache {
        inner: Mutex<HashMap<(String, String, u32), Vec<u8>>>,
    }

    impl PageCache for MemCache {
        fn has(&self, source_id: &str, content_id: &str, page: u32) -> bool {
            self.inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(&(source_id.into(), content_id.into(), page))
        }
        fn put(
            &self,
            source_id: &str,
            content_id: &str,
            page: u32,
            bytes: &[u8],
        ) -> Result<(), AppError> {
            self.inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert((source_id.into(), content_id.into(), page), bytes.to_vec());
            Ok(())
        }
        fn remove_content(&self, _source_id: &str, _content_id: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn path(&self, source_id: &str, content_id: &str, page: u32) -> Result<PathBuf, AppError> {
            if source_id.contains("..") || content_id.is_empty() {
                return Err(AppError::Validation("buruk".into()));
            }
            Ok(PathBuf::from(format!(
                "/cache/{source_id}/{content_id}/page_{page}.jpg"
            )))
        }
    }

    fn usecase(payload: &[u8], fail: bool) -> CachePageImageUseCase<ScriptedFetcher, MemCache> {
        CachePageImageUseCase::new(
            ScriptedFetcher {
                payload: payload.to_vec(),
                fail,
                calls: Mutex::new(0),
            },
            MemCache {
                inner: Mutex::new(HashMap::new()),
            },
        )
    }

    #[tokio::test]
    async fn miss_fetch_put_lalu_hit() {
        let uc = usecase(b"IMG", false);
        let r = uc
            .execute("nh", "g1", 3, "https://cdn.test/3.webp")
            .await
            .unwrap();
        assert_eq!(
            r,
            PageImageResult::Cached("/cache/nh/g1/page_3.jpg".to_string())
        );
        // Kedua: hit tanpa fetch ulang (tetap Cached).
        let r2 = uc
            .execute("nh", "g1", 3, "https://cdn.test/3.webp")
            .await
            .unwrap();
        assert_eq!(r2, r);
        assert_eq!(
            *uc.fetcher.calls.lock().unwrap_or_else(|e| e.into_inner()),
            1
        );
    }

    #[tokio::test]
    async fn fetch_gagal_fallback_remote() {
        let uc = usecase(b"IMG", true);
        let r = uc
            .execute("nh", "g1", 0, "https://cdn.test/0.jpg")
            .await
            .unwrap();
        assert_eq!(
            r,
            PageImageResult::Remote("https://cdn.test/0.jpg".to_string())
        );
    }

    #[tokio::test]
    async fn bytes_kosong_fallback_remote() {
        let uc = usecase(b"", false);
        let r = uc
            .execute("nh", "g1", 0, "https://cdn.test/0.jpg")
            .await
            .unwrap();
        assert_eq!(
            r,
            PageImageResult::Remote("https://cdn.test/0.jpg".to_string())
        );
    }

    #[tokio::test]
    async fn input_buruk_validation_bukan_fallback() {
        let uc = usecase(b"IMG", false);
        assert!(uc
            .execute("", "g1", 0, "https://cdn.test/0.jpg")
            .await
            .is_err());
        assert!(uc
            .execute("nh", "g1", 0, "file:///etc/passwd")
            .await
            .is_err());
    }
}
