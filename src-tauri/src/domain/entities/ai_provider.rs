//! AI provider BYOK — spec `settings-ai` (9.2).
//! Metadata provider disimpan di KV; kunci API HANYA di keychain OS
//! (`has_key` = penanda, kirimannya tak pernah ikut ke UI).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Jenis provider AI (preset OpenAI/Gemini/Cohere + kustom OpenAI-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum AiProviderKind {
    OpenAi,
    Gemini,
    Cohere,
    Custom,
}

impl AiProviderKind {
    /// Base URL bawaan per jenis (kosong = kustom wajib diisi pengguna).
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::Cohere => "https://api.cohere.com/v2",
            Self::Custom => "",
        }
    }

    /// Model bawaan per jenis.
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-4o-mini",
            Self::Gemini => "gemini-2.0-flash",
            Self::Cohere => "command-r",
            Self::Custom => "",
        }
    }

    /// Endpoint daftar model (LOV) per jenis — port `AiProviderType.modelsUrl`
    /// mobile. `Custom` tanpa endpoint: input manual saja.
    pub fn models_url(&self) -> Option<&'static str> {
        match self {
            Self::OpenAi => Some("https://api.openai.com/v1/models"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com/v1beta/models"),
            // Mobile listing pakai v1 (bentuk native `{models:[…]}`), bukan v2 chat.
            Self::Cohere => Some("https://api.cohere.com/v1/models"),
            Self::Custom => None,
        }
    }
}

/// Metadata provider TANPA kunci — siap ditampilkan di daftar UI.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub kind: AiProviderKind,
    pub base_url: String,
    pub model: String,
    /// True bila kunci tersimpan di keychain OS. Kunci asli tak pernah
    /// di-serialize ke frontend.
    pub has_key: bool,
}

/// Input simpan provider dari UI; kunci dikirim terpisah di command
/// (`api_key`), bukan lewat entity ini.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AiProviderInput {
    pub id: Option<String>,
    pub name: String,
    pub kind: AiProviderKind,
    pub base_url: String,
    pub model: String,
}

/// Satu entri model untuk LOV pengaturan — port `AiModelOption` mobile.
/// `is_vision`: `Some(true)` vision, `Some(false)` teks-saja, `None` tak
/// diketahui (API tanpa flag → tanpa badge).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AiModelOption {
    /// ID model sesuai daftar API provider.
    pub id: String,
    /// Label human-readable; `None` = tampilkan `id`.
    pub label: Option<String>,
    pub is_vision: Option<bool>,
}

impl AiModelOption {
    /// Tampilan default untuk UI (`label` bila ada, else `id`).
    pub fn display_label(&self) -> &str {
        self.label
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.id)
    }
}
