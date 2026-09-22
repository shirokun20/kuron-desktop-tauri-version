//! Repo traits — port `lib/domain/repositories/*`. Dipegang AppState
//! sebagai `Arc<dyn>`; async nyusul 3.3 bareng SQLite/reqwest.

pub mod ai_provider_repository;
pub mod content_repository;
pub mod downloads_repository;
pub mod library_repository;
pub mod page_cache;
pub mod page_fetcher;

pub use ai_provider_repository::AiProviderRepository;
pub use content_repository::{ContentRepository, HomeFeedRepository};
pub use downloads_repository::DownloadsRepository;
pub use library_repository::LibraryRepository;
pub use page_cache::PageCache;
pub use page_fetcher::PageFetcher;
