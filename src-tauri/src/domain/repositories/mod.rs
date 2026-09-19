//! Repo traits — port `lib/domain/repositories/*`. Sync Fase 0 (mock);
//! async + `Arc<dyn>` nyusul Fase 1 bareng SQLite/reqwest.

pub mod content_repository;

pub use content_repository::{ContentRepository, HomeFeedRepository};
