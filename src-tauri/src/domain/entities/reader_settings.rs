//! ReaderSettings — port `entities/reader_settings_entity.dart`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadingMode {
    Paginated,
    ContinuousScroll,
    Webtoon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderSettings {
    pub mode: ReadingMode,
    pub right_to_left: bool,
}
