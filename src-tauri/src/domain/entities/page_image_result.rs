//! PageImageResult sealed — port `entities/page_image_result.dart`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "value")]
pub enum PageImageResult {
    Cached(String),
    Remote(String),
}
