//! Application services — port `services/*` mobile (spec §9, §13).

pub mod download_manager;
pub mod rate_limiter;

pub use download_manager::{DownloadEventSink, DownloadManager, NullDownloadSink};
pub use rate_limiter::RateLimiter;
