//! AppState: composition root payload, di-`manage` di lib.rs.
//! Ganti `get_it` `service_locator.dart`. Repo concrete dibungkus
//! `Arc<dyn Trait>` agar command resolve dari state, bukan konstruksi sendiri.

use std::sync::Arc;

use crate::{
    data::MockContentRepository, domain::repositories::ContentRepository,
};

pub struct AppState {
    pub app_name: String,
    pub version: String,
    pub content_repo: Arc<dyn ContentRepository>,
}

impl AppState {
    pub fn new(content_repo: Arc<dyn ContentRepository>) -> Self {
        Self {
            app_name: "Kuron Desktop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            content_repo,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Arc::new(MockContentRepository))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::HomeFeedRepository;

    #[test]
    fn default_state_serves_mock_feed() {
        let feed =
            tauri::async_runtime::block_on(AppState::default().content_repo.home_feed())
                .unwrap();
        assert_eq!(feed.len(), 8);
    }
}
