//! Model catalog (LOV) — port `ai_model_catalog.dart` mobile.
//! Live `GET /models` per jenis provider; TANPA daftar hardcode dan
//! TANPA fallback: gagal → error, UI menampilkan retry + input manual.
//! Parsing murni (tanpa network) supaya bisa diuji offline.

use crate::{
    core::AppError,
    domain::{AiModelOption, AiProviderKind},
    network::HttpClientManager,
};

/// Ambil daftar model live untuk satu jenis provider.
/// Kunci hanya dipakai di sini (header Bearer / query Gemini) — tak disimpan.
pub async fn fetch_models(
    http: &HttpClientManager,
    kind: AiProviderKind,
    api_key: &str,
) -> Result<Vec<AiModelOption>, AppError> {
    let url = kind
        .models_url()
        .ok_or_else(|| AppError::Validation("provider kustom tanpa daftar model".into()))?;
    let key = api_key.trim();
    if key.is_empty() {
        return Err(AppError::Validation(
            "kunci API wajib diisi sebelum memuat daftar model".into(),
        ));
    }

    // Gemini: kunci sebagai query `?key=`; sisanya header Bearer (1:1 mobile).
    let is_gemini = kind == AiProviderKind::Gemini;
    let url = if is_gemini {
        format!("{url}?key={key}")
    } else {
        url.to_string()
    };
    let mut req = http.client().get(url).header("Accept", "application/json");
    if !is_gemini {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let body = req
        .send()
        .await
        .map_err(|e| AppError::Network(format!("daftar model: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Network(format!("daftar model: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::Network(format!("daftar model: {e}")))?;

    parse_models(kind, &body)
}

/// Parse body daftar model per bentuk provider (murni, tanpa network).
pub fn parse_models(kind: AiProviderKind, body: &str) -> Result<Vec<AiModelOption>, AppError> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::Network(format!("format daftar model tak terduga: {e}")))?;
    match kind {
        AiProviderKind::Gemini => parse_gemini(&json),
        AiProviderKind::Cohere => {
            // Bentuk native Cohere `{models:[…]}`; fallback gaya OpenAI `{data:[…]}`.
            if json.get("models").is_some() {
                parse_cohere_native(&json)
            } else {
                parse_openai_compatible(&json, kind)
            }
        }
        AiProviderKind::OpenAi | AiProviderKind::Custom => parse_openai_compatible(&json, kind),
    }
}

/// Gaya OpenAI: `{object, data:[{id, name?}]}` — dipakai OpenAI (+ fallback).
/// Flag vision hanya dari API (mis. `architecture.input_modalities` OpenRouter);
/// tanpa flag → `None` (tanpa menebak nama).
fn parse_openai_compatible(
    json: &serde_json::Value,
    kind: AiProviderKind,
) -> Result<Vec<AiModelOption>, AppError> {
    let data = json
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Network("format daftar model tak terduga (data hilang)".into()))?;
    let mut out = Vec::new();
    for item in data {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let label = item
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != id)
            .map(str::to_string);
        out.push(AiModelOption {
            id: id.to_string(),
            label,
            is_vision: openai_vision_flag(item, kind),
        });
    }
    ensure_nonempty(out)
}

fn openai_vision_flag(item: &serde_json::Value, kind: AiProviderKind) -> Option<bool> {
    match kind {
        AiProviderKind::Cohere => item
            .get("features")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|f| f.as_str() == Some("vision"))),
        // OpenRouter: `architecture.input_modalities` berisi "image" → vision;
        // ada modalities tanpa image → teks-saja; absen → tak diketahui.
        _ => {
            let mods = item
                .get("architecture")
                .and_then(|a| a.get("input_modalities"))
                .and_then(|v| v.as_array())?;
            Some(mods.iter().any(|m| m.as_str() == Some("image")))
        }
    }
}

/// Bentuk native Cohere: `{models:[{name, endpoints, features}]}`.
/// Hanya model ber-endpoint `chat`; id = `name`; vision dari `features`.
fn parse_cohere_native(json: &serde_json::Value) -> Result<Vec<AiModelOption>, AppError> {
    let models = json
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AppError::Network("format daftar model tak terduga (models hilang)".into())
        })?;
    let mut out = Vec::new();
    for item in models {
        let Some(id) = item.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let endpoints = item.get("endpoints").and_then(|v| v.as_array());
        if let Some(eps) = endpoints {
            if !eps.iter().any(|e| e.as_str() == Some("chat")) {
                continue;
            }
        }
        let is_vision = item
            .get("features")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|f| f.as_str() == Some("vision")));
        out.push(AiModelOption {
            id: id.to_string(),
            label: None,
            is_vision,
        });
    }
    ensure_nonempty(out)
}

/// Gemini: `{models:[{name:"models/<id>", displayName?, supportedGenerationMethods}]}`.
/// Hanya yang `generateContent`; buang embedding; strip prefix `models/`;
/// vision = true (generateContent-capable) — 1:1 `GeminiCatalogParser`.
fn parse_gemini(json: &serde_json::Value) -> Result<Vec<AiModelOption>, AppError> {
    let models = json
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AppError::Network("format daftar model tak terduga (models hilang)".into())
        })?;
    let mut out = Vec::new();
    for item in models {
        let raw = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let id = raw.strip_prefix("models/").unwrap_or(raw);
        if id.is_empty() {
            continue;
        }
        let methods = item
            .get("supportedGenerationMethods")
            .and_then(|v| v.as_array());
        let supports_generate = methods
            .map(|ms| ms.iter().any(|m| m.as_str() == Some("generateContent")))
            .unwrap_or(false);
        if !supports_generate {
            continue;
        }
        if id.to_lowercase().contains("embedding") {
            continue;
        }
        let label = item
            .get("displayName")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != id)
            .map(str::to_string);
        out.push(AiModelOption {
            id: id.to_string(),
            label,
            is_vision: Some(true),
        });
    }
    ensure_nonempty(out)
}

fn ensure_nonempty(mut out: Vec<AiModelOption>) -> Result<Vec<AiModelOption>, AppError> {
    if out.is_empty() {
        return Err(AppError::Network("daftar model kosong".into()));
    }
    out.sort_by(|a, b| {
        a.display_label()
            .to_lowercase()
            .cmp(&b.display_label().to_lowercase())
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPENAI_BODY: &str = r#"{"object":"list","data":[
        {"id":"gpt-4o","name":"GPT-4o"},
        {"id":"gpt-4o-mini"},
        {"id":""}
    ]}"#;

    const COHERE_BODY: &str = r#"{"models":[
        {"name":"command-r","endpoints":["chat"],"features":[]},
        {"name":"command-r-vision","endpoints":["chat"],"features":["vision"]},
        {"name":"embed-english","endpoints":["embed"],"features":[]}
    ]}"#;

    const GEMINI_BODY: &str = r#"{"models":[
        {"name":"models/gemini-2.0-flash","displayName":"Gemini 2.0 Flash",
         "supportedGenerationMethods":["generateContent","countTokens"]},
        {"name":"models/text-embedding-004",
         "supportedGenerationMethods":["embedContent"]},
        {"name":"models/gemini-1.5-pro","supportedGenerationMethods":["generateContent"]}
    ]}"#;

    #[test]
    fn openai_data_list_parses_and_skips_blank_id() {
        let out = parse_models(AiProviderKind::OpenAi, OPENAI_BODY).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "gpt-4o");
        assert_eq!(out[0].label.as_deref(), Some("GPT-4o"));
        // urutan abjad per display label
        assert_eq!(out[1].id, "gpt-4o-mini");
        assert_eq!(out[1].label, None);
        assert_eq!(out[0].is_vision, None);
    }

    #[test]
    fn cohere_native_filters_non_chat_and_flags_vision() {
        let out = parse_models(AiProviderKind::Cohere, COHERE_BODY).unwrap();
        assert_eq!(out.len(), 2);
        let vision = out.iter().find(|m| m.id == "command-r-vision").unwrap();
        assert_eq!(vision.is_vision, Some(true));
        let text = out.iter().find(|m| m.id == "command-r").unwrap();
        assert_eq!(text.is_vision, Some(false));
        assert!(!out.iter().any(|m| m.id == "embed-english"));
    }

    #[test]
    fn gemini_keeps_generate_content_and_strips_prefix() {
        let out = parse_models(AiProviderKind::Gemini, GEMINI_BODY).unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|m| !m.id.starts_with("models/")));
        assert!(!out.iter().any(|m| m.id.contains("embedding")));
        let flash = out.iter().find(|m| m.id == "gemini-2.0-flash").unwrap();
        assert_eq!(flash.label.as_deref(), Some("Gemini 2.0 Flash"));
        assert_eq!(flash.is_vision, Some(true));
    }

    #[test]
    fn empty_list_is_error_no_fallback() {
        let err = parse_models(AiProviderKind::OpenAi, r#"{"data":[]}"#).unwrap_err();
        assert!(err.to_string().contains("kosong"));
    }

    #[test]
    fn bad_json_is_error() {
        let err = parse_models(AiProviderKind::OpenAi, "<html>403</html>").unwrap_err();
        assert!(err.to_string().contains("tak terduga"));
    }
}
