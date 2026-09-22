//! HttpPageFetcher — `PageFetcher` di atas `HttpClientManager` (8.1).

use std::sync::Arc;

use async_trait::async_trait;

use crate::{core::AppError, domain::repositories::PageFetcher, network::HttpClientManager};

pub struct HttpPageFetcher {
    http: HttpClientManager,
}

impl HttpPageFetcher {
    pub fn new(http: HttpClientManager) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PageFetcher for HttpPageFetcher {
    async fn fetch(
        &self,
        url: &str,
        source_id: &str,
        referer: Option<&str>,
    ) -> Result<Vec<u8>, AppError> {
        self.http.get_bytes(url, source_id, referer).await
    }
}

/// Convenience: `Arc<HttpPageFetcher>` sebagai `Arc<dyn PageFetcher>`.
pub fn http_fetcher_arc(http: HttpClientManager) -> Arc<dyn PageFetcher> {
    Arc::new(HttpPageFetcher::new(http))
}
