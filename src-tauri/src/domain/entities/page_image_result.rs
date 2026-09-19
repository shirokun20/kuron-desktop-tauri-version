//! PageImageResult sealed — port `entities/page_image_result.dart`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PageImageResult {
    Cached(String),
    Remote(String),
}
