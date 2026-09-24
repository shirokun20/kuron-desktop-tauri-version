//! Data / Infrastructure — depends on Domain only (spec §9).
//! Fase 0: mock impl. Remote/local real nyusul Fase 2.

pub mod datasources;
pub mod models;
pub mod native;
pub mod repositories;
pub mod services;

pub use native::bubble_detector::BubbleDetector;
pub use repositories::content_repository_impl::{ContentRepositoryImpl, MockContentRepository};
pub use services::http_fetcher_arc;
