//! AI translation entities — port `entities/ai_translation.dart`.
//! `BubbleBox { rect, polygon }` + `TranslationStyle`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BubbleBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub translated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTranslation {
    pub page_index: u32,
    pub bubbles: Vec<BubbleBox>,
}
