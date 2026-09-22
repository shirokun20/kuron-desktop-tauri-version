//! AiProviderRepository trait — BYOK 9.2. Kunci API hanya lewat backend
//! (keychain OS); frontend cuma lihat `has_key`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    core::AppError,
    domain::{AiProvider, AiProviderInput},
};

#[async_trait]
pub trait AiProviderRepository: Send + Sync {
    /// Daftar provider + status kunci (tak pernah menyertakan kunci asli).
    async fn list(&self) -> Result<Vec<AiProvider>, AppError>;
    /// Simpan/upsert metadata + kunci ke keychain; kembali metadata siap UI.
    async fn save(&self, input: AiProviderInput, api_key: &str) -> Result<AiProvider, AppError>;
    /// Hapus provider: metadata + kunci. False bila id tak dikenal.
    async fn delete(&self, id: &str) -> Result<bool, AppError>;
    /// Baca kunci dari keychain — hanya untuk use case translate backend
    /// (7.2); TIDAK diekspos ke command/UI.
    async fn get_key(&self, id: &str) -> Result<Option<String>, AppError>;
}

/// Blanket impl agar `Arc<dyn AiProviderRepository>` (isi AppState)
/// bisa dipakai langsung oleh use case generik.
#[async_trait]
impl<R: AiProviderRepository + ?Sized> AiProviderRepository for Arc<R> {
    async fn list(&self) -> Result<Vec<AiProvider>, AppError> {
        (**self).list().await
    }

    async fn save(&self, input: AiProviderInput, api_key: &str) -> Result<AiProvider, AppError> {
        (**self).save(input, api_key).await
    }

    async fn delete(&self, id: &str) -> Result<bool, AppError> {
        (**self).delete(id).await
    }

    async fn get_key(&self, id: &str) -> Result<Option<String>, AppError> {
        (**self).get_key(id).await
    }
}

/// Blanket impl agar `&R` (umum di test use case) juga memenuhi trait.
#[async_trait]
impl<'a, R: AiProviderRepository + ?Sized> AiProviderRepository for &'a R {
    async fn list(&self) -> Result<Vec<AiProvider>, AppError> {
        (**self).list().await
    }

    async fn save(&self, input: AiProviderInput, api_key: &str) -> Result<AiProvider, AppError> {
        (**self).save(input, api_key).await
    }

    async fn delete(&self, id: &str) -> Result<bool, AppError> {
        (**self).delete(id).await
    }

    async fn get_key(&self, id: &str) -> Result<Option<String>, AppError> {
        (**self).get_key(id).await
    }
}
