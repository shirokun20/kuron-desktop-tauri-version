//! Data / Infrastructure — depends on Domain only (spec §9).
//! Fase 0: mock impl. Remote/local real nyusul Fase 2.

pub mod datasources;
pub mod models;
pub mod native;
pub mod repositories;

pub use repositories::content_repository_impl::{ContentRepositoryImpl, MockContentRepository};
