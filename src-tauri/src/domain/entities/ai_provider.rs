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
