//! Tag — metadata konten (port `Tag` kuron_core mobile).
//! Tipe nhentai: tag, artist, character, parody, group, language, category.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Satu tag: id string (nh numerik → string, MD UUID) + nama + tipe + jumlah.
/// Mobile: chip tag bawa ID stabil (UUID authorOrArtist) untuk navigasi.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub tag_type: String,
    pub count: i64,
}
