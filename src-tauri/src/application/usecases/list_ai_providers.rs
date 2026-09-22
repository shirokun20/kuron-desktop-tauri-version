//! ListAiProvidersUseCase — daftar provider + status kunci (9.2).

use crate::{
    core::AppError,
    domain::{repositories::AiProviderRepository, AiProvider},
};

pub struct ListAiProvidersUseCase<R> {
    repo: R,
}

impl<R: AiProviderRepository> ListAiProvidersUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<Vec<AiProvider>, AppError> {
        self.repo.list().await
    }
}
