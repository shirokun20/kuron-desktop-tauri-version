//! GetHomeFeedUseCase — feed halaman utama (splash -> main).
//! Fase 0: repo mock. Fase 1: `ContentRepository::search` real.

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

    pub fn execute(&self) -> Result<Vec<Content>, AppError> {
        self.repo.home_feed()
    }
}
