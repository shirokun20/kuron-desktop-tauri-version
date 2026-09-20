//! ReaderSettings — port `entities/reader_settings_entity.dart`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum ReadingMode {
    Paginated,
    ContinuousScroll,
    Webtoon,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ReaderSettings {
    pub mode: ReadingMode,
    pub right_to_left: bool,
}
