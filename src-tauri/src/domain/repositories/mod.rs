//! Repo traits — port `lib/domain/repositories/*`. Dipegang AppState
//! sebagai `Arc<dyn>`; async nyusul 3.3 bareng SQLite/reqwest.

pub mod ai_provider_repository;
pub mod content_repository;
pub mod library_repository;

pub use ai_provider_repository::AiProviderRepository;
pub use content_repository::{ContentRepository, HomeFeedRepository};
pub use library_repository::LibraryRepository;
