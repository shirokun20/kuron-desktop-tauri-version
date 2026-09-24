//! AI translation entities — port `entities/ai_translation.dart` +
//! `kuron_native/lib/src/bubble_box.dart` (5.2: confidence/shape/kind/tail).
//! `BubbleBox { rect, polygon }` + `TranslationStyle`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BubbleBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Skor deteksi ONNX 0..1 (1.0 = manual, port `BubbleBox.confidence`).
    pub confidence: f64,
    /// Outline poligon [[x,y],...] koordinat piksel asli; None = box fallback.
    pub shape: Option<Vec<Vec<i32>>>,
    /// Kelas: "balloon" | "text" | "frame" | "unknown".
    pub kind: Option<String>,
    /// Ekor gambar-user [[x,y],...]; dipakai draw mode 7.3 (manual bubble).
    pub tail: Option<Vec<Vec<i32>>>,
    pub translated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PageTranslation {
    pub page_index: u32,
    pub bubbles: Vec<TranslatedBubble>,
    /// Bahasa terdeteksi (mis. "ja"); kosong bila tak diketahui.
    pub detected_lang: String,
    /// True bila jalur full-image (tanpa bubble detector).
    pub used_fallback: bool,
}

/// Satu bubble terjemahan — port `BubbleTranslation` mobile (7.2).
/// Koordinat piksel ruang gambar asli; `shape` ditempel ulang pasca-AI
/// dari hasil deteksi ONNX (tak selamat lewat round-trip mosaic).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TranslatedBubble {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Teks asli di bubble (untuk belajar/glosarium); kosong bila tak ada.
    pub original: String,
    /// Bacaan latin (romaji/romanisasi); kosong bila asli sudah latin.
    pub reading: String,
    pub translated: String,
    /// Poligon outline [[x,y],...]; None = fallback box.
    pub shape: Option<Vec<Vec<i32>>>,
    /// Teks di atas artwork ramai — render patch putih di belakang teks
    /// (heuristik "bubble flat" cypy, dihitung pasca-AI).
    pub needs_white_patch: bool,
}

/// Gaya/nada terjemahan — disuntik ke prompt AI (port `TranslationStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum TranslationStyle {
    Natural,
    Genz,
    Action,
    Romantis,
    Formal,
    Kasar,
    Literal,
}

impl TranslationStyle {
    /// Instruksi gaya untuk prompt (port `TranslationStyle.instruction`).
    pub fn instruction(&self) -> &'static str {
        match self {
            Self::Natural => {
                "Use natural, fluent Indonesian. Match tone to context. Default style."
            }
            Self::Genz => {
                "Use informal Indonesian slang: gue/lo, sih/dong/nih/deh/doang. For comedy/light romance."
            }
            Self::Action => {
                "Short, punchy sentences. Exclamations: Hah!, Hragh!, Mati lo!. For battle series."
            }
            Self::Romantis => {
                "Soft, poetic. Aku/Kamu. Light metaphor. For romance/drama."
            }
            Self::Formal => {
                "Standard literary Indonesian for narrators, mystery, horror, wise characters."
            }
            Self::Kasar => {
                "Blunt. anjir/bangsat/kampret/goblok. (18+, requires confirmation)"
            }
            Self::Literal => {
                "Word-for-word accurate. Keep honorifics. For learning / checking meaning."
            }
        }
    }
}
