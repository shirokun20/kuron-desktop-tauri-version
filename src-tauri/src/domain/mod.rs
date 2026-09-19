//! Domain: pure, zero deps (spec §8). No Tauri/sqlite/reqwest imports.

pub mod entities;

pub use entities::hello::Hello;
