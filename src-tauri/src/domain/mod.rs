//! Domain: pure, zero deps (spec §8). No Tauri/sqlite/reqwest imports.
//! Port `lib/domain/entities/*` (Freezed) -> Rust structs + serde.

pub mod entities;
pub mod repositories;
pub mod services;
pub mod value_objects;

pub use entities::{
    ai_translation::{BubbleBox, PageTranslation},
    chapter::Chapter,
    content::Content,
    download_task::{DownloadState, DownloadTask},
    glossary::GlossaryEntry,
    hello::Hello,
    page_image_result::PageImageResult,
    reader_settings::{ReaderSettings, ReadingMode},
    search_filter::SearchFilter,
};
pub use value_objects::{language::Language, source_id::SourceId};
