//! GetHomeFeedUseCase — feed halaman utama (splash -> main).
//! Fase 1: trait async. Fase 2: `ContentRepository::search` real.

use crate::{
    core::AppError,
    domain::{repositories::HomeFeedRepository, Content},
};

pub struct GetHomeFeedUseCase<R> {
    repo: R,
}

impl<R: HomeFeedRepository> GetHomeFeedUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, page: u32) -> Result<Vec<Content>, AppError> {
        self.repo.home_feed(page).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MockContentRepository;

    #[test]
    fn serves_mock_feed_via_arc_dyn() {
        let repo: std::sync::Arc<dyn HomeFeedRepository> =
            std::sync::Arc::new(MockContentRepository);
        let feed = tauri::async_runtime::block_on(GetHomeFeedUseCase::new(repo).execute(1))
            .unwrap();
        assert_eq!(feed.len(), 8);
    }
}
