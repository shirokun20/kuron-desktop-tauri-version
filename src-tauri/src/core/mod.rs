//! Kuron core: error, config, constants (spec §9 Core).
//! Pure Rust, no Tauri/Svelte imports.

pub mod config;
pub mod constants;
pub mod di;
pub mod error;
pub mod logger;

pub use di::{AppState, InstalledSource};
pub use error::AppError;
