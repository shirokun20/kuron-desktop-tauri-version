//! Tag — metadata konten (port `Tag` kuron_core mobile).
//! Tipe nhentai: tag, artist, character, parody, group, language, category.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Satu tag: id numerik + nama + tipe + jumlah pakai.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub tag_type: String,
    pub count: i64,
}
