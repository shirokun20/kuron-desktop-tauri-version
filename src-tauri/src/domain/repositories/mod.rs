//! Repo traits — port `lib/domain/repositories/*`. Dipegang AppState
//! sebagai `Arc<dyn>`; async nyusul 3.3 bareng SQLite/reqwest.

pub mod content_repository;

pub use content_repository::{ContentRepository, HomeFeedRepository};
