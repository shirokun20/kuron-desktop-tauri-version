//! DeleteAiProviderUseCase — hapus metadata + kunci provider (9.2).

use crate::{core::AppError, domain::repositories::AiProviderRepository};

pub struct DeleteAiProviderUseCase<R> {
    repo: R,
}

impl<R: AiProviderRepository> DeleteAiProviderUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// True bila id dikenal dan terhapus.
    pub async fn execute(&self, id: &str) -> Result<bool, AppError> {
        if id.trim().is_empty() {
            return Err(AppError::Validation("id provider wajib diisi".into()));
        }
        self.repo.delete(id.trim()).await
    }
}
