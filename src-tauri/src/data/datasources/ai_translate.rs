//! AI translate provider — port `openai_compatible_provider.dart`,
//! `gemini_translation_provider.dart`, `cohere_translation_provider.dart`
//! + `model_json_parser.dart` (7.2).
//! Alur: mosaic berlabel (5.1 `build_mosaic`) → prompt STRICT JSON → parse
//! kebal (fences/ellipsis/koma) → petakan per nomor → `TranslatedBubble`
//! (shape ditempel ulang di use case dari deteksi ONNX).
//! Kunci API hanya lewat memori backend (keychain, 9.2) — tak pernah ke FE.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::{
    core::AppError,
    domain::{AiProvider, AiProviderKind, PageTranslation, TranslatedBubble, TranslationStyle},
    network::HttpClientManager,
};

/// Prompt mosaic berlabel (port `_mosaicPrompt…` ketiga provider, 1:1).
pub fn mosaic_prompt(
    target_lang: &str,
    style: TranslationStyle,
    skip_sfx: bool,
    reading_direction: &str,
    glossary: Option<&str>,
) -> String {
    let sfx = if skip_sfx {
        "Return \"SKIP\" for any bubble containing only sound effects (ドドド, バキ, ガシャン, etc.)."
    } else {
        "Translate ALL bubbles including sound effects (no SKIP for SFX)."
    };
    let mut p = format!(
        "Translate the manga/manhwa image. Each bubble has a red number ID on its left.\n\
         Reading order: bubbles numbered {reading_direction}, top-to-bottom.\n\
         Return STRICT JSON (no markdown, no comments) with numeric string keys:\n\
         {{\"1\": {{\"original\": \"<text in bubble 1>\", \"reading\": \"<latin reading 1>\", \"translated\": \"<translation 1>\"}}, \"2\": \"SKIP\", \"3\": {{\"original\": \"<text in bubble 3>\", \"reading\": \"<latin reading 3>\", \"translated\": \"<translation 3>\"}}}}\n\
         Rules:\n\
         - Map each number to the text inside that bubble, in reading order.\n\
         - \"original\" = the exact text inside the bubble (for learning/glossary).\n\
         - \"reading\" = Latin reading of the original text: romaji for Japanese, romanization for Korean/Chinese/other scripts (helps pronunciation). Empty if original is already Latin.\n\
         - \"translated\" = the translation into {target_lang}.\n\
         - SKIP if a bubble is a sound effect (ドドド, バキ, etc.).\n\
         - Keep honorifics (-san, -kun, -chan) as-is.\n\
         - Return ALL visible IDs.\n\
         - Output ONLY that JSON for the visible numbered bubbles — never output \"...\", placeholders like <...>, or explanations.\n\
         - Style: {style}\n\
         {sfx}\n",
        style = style.instruction(),
    );
    if let Some(g) = glossary.map(str::trim).filter(|g| !g.is_empty()) {
        if !p.ends_with('\n') {
            p.push('\n');
        }
        p.push_str(g);
        p.push('\n');
    }
    p
}

/// Prompt full-image fallback: koordinat persen (port `_fullImagePrompt`).
pub fn full_image_prompt(target_lang: &str, style: TranslationStyle, skip_sfx: bool) -> String {
    format!(
        "Translate the manga/manhwa page image into {target_lang}.\n\
         Return STRICT JSON (no markdown, no comments) in this exact shape:\n\
         [{{\"x\": <left %>, \"y\": <top %>, \"w\": <width %>, \"h\": <height %> \"translated\": \"...\"}}]\n\
         Rules:\n\
         - Return coordinates as PERCENTAGES (0-100) of the image size.\n\
         - \"translated\" = translation in {target_lang}. Use \"SKIP\" for sound-effect-only text.\n\
         - Keep honorifics (-san, -kun, -chan) as-is.\n\
         - Style: {style}\n\
         {sfx}\n",
        style = style.instruction(),
        sfx = if skip_sfx {
            "Return \"SKIP\" for any bubble containing only sound effects."
        } else {
            "Translate ALL bubbles including sound effects."
        },
    )
}

/// True bila terjemahan adalah gema template, bukan hasil nyata
/// (port `looksLikePlaceholder`: `<...>` utuh atau `...` telanjang).
pub fn looks_like_placeholder(v: &str) -> bool {
    let t = v.trim();
    if t == "..." {
        return true;
    }
    t.len() >= 3
        && t.starts_with('<')
        && t.ends_with('>')
        && !t[1..t.len() - 1].contains(['<', '>'])
}

/// Kandidat ekstraksi JSON ala `ModelJsonParser._candidates`:
/// span seimbang tiap opener → span naive → repair ellipsis/koma.
fn candidates(content: &str, open: char, close: char) -> Vec<String> {
    let text = strip_fences(content.trim());
    let mut out: Vec<String> = Vec::new();
    let add = |out: &mut Vec<String>, s: &str| {
        if !s.is_empty() && !out.iter().any(|e| e == s) {
            out.push(s.to_string());
        }
    };
    for b in balanced_candidates(&text, open, close) {
        add(&mut out, &b);
    }
    if let (Some(s), Some(e)) = (text.find(open), text.rfind(close)) {
        if e > s {
            add(&mut out, &text[s..=e]);
        }
    }
    for base in out.clone() {
        let no_ellipsis = remove_ellipsis(&base);
        add(&mut out, &no_ellipsis);
        let repaired = remove_trailing_commas(&no_ellipsis);
        add(&mut out, &repaired);
    }
    out
}

fn strip_fences(text: &str) -> String {
    let mut t = text.to_string();
    if t.starts_with("```") {
        if let Some(nl) = t.find('\n') {
            t = t[nl + 1..].to_string();
        }
        if let Some(end) = t.rfind("```") {
            t = t[..end].to_string();
        }
        t = t.trim().to_string();
    }
    t
}

fn balanced_candidates(text: &str, open: char, close: char) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut from = 0;
    while out.len() < 8 {
        let Some(rel) = chars[from..].iter().position(|&c| c == open) else {
            break;
        };
        let start = from + rel;
        if let Some(span) = extract_balanced(&chars, start, open, close) {
            if !out.contains(&span) {
                out.push(span);
            }
        }
        from = start + 1;
    }
    out
}

fn extract_balanced(chars: &[char], start: usize, open: char, close: char) -> Option<String> {
    let (mut depth, mut in_str, mut esc) = (0usize, false, false);
    for (i, &ch) in chars.iter().enumerate().skip(start) {
        if in_str {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            in_str = true;
        } else if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(chars[start..=i].iter().collect());
            }
        }
    }
    None
}

fn remove_ellipsis(text: &str) -> String {
    // Hapus penanda `...` contoh prompt (port `_ellipsisRegExp`).
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.' && i + 2 < chars.len() && chars[i + 1] == '.' && chars[i + 2] == '.' {
            // Telan spasi/koma sekitar (`, ...` / `... ,`).
            while out.ends_with([' ', '\t', '\n']) {
                out.pop();
            }
            i += 3;
            while i < chars.len() && matches!(chars[i], ' ' | '\t' | '\n' | ',') {
                // Biarkan SATU koma penutup struktur? tidak — telan semua,
                // repair koma menangani sisa.
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn remove_trailing_commas(text: &str) -> String {
    // `,}` → `}`, `,]` → `]` (port `_trailingCommaRegExp`).
    let mut out = text.to_string();
    for (bad, good) in [(",}", "}"), (",]", "]")] {
        while out.contains(bad) {
            out = out.replacen(bad, good, 1);
        }
    }
    out
}

fn unusable() -> AppError {
    AppError::Network(
        "Model mengembalikan respons yang tak bisa dipakai. Coba lagi, atau ganti ke model vision yang lebih kuat.".into(),
    )
}

/// Parse JSON objek mosaic `{"1": {...}}` (port `parseMosaicJson`).
pub fn parse_mosaic_json(
    content: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    for cand in candidates(content, '{', '}') {
        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&cand)
        {
            if map.is_empty() || map.keys().any(|k| k.parse::<u32>().is_ok()) {
                return Ok(map);
            }
        }
    }
    Err(unusable())
}

/// Parse array koordinat full-image (port `parseJsonArray`).
pub fn parse_json_array(content: &str) -> Result<Vec<serde_json::Value>, AppError> {
    for cand in candidates(content, '[', ']') {
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&cand)
        {
            return Ok(arr);
        }
    }
    Err(unusable())
}

/// Satu bubble input (rect piksel asli, urutan baca).
#[derive(Debug, Clone, Copy)]
pub struct MosaicBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Petakan hasil mosaic per nomor → bubble (SKIP/placeholder dibuang,
/// port `_mapMosaicResult`). `shapes[i]` = poligon ONNX box ke-i (opsional).
pub fn map_mosaic_result(
    parsed: &serde_json::Map<String, serde_json::Value>,
    boxes: &[MosaicBox],
    shapes: &[Option<Vec<Vec<i32>>>],
    lang: &str,
) -> Result<PageTranslation, AppError> {
    let mut out = Vec::new();
    let mut placeholder_skips = 0;
    for (i, b) in boxes.iter().enumerate() {
        let raw = parsed.get(&format!("{}", i + 1));
        let (mut translated, mut original, mut reading) = (None, String::new(), String::new());
        match raw {
            Some(serde_json::Value::Object(m)) => {
                translated = m
                    .get("translated")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string());
                original = m
                    .get("original")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                reading = m
                    .get("reading")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
            }
            Some(v) if v.is_string() => {
                translated = v.as_str().map(|s| s.trim().to_string());
            }
            _ => {}
        }
        let t = match translated {
            Some(t) if !t.is_empty() && t.to_uppercase() != "SKIP" => t,
            _ => continue,
        };
        if looks_like_placeholder(&t) {
            placeholder_skips += 1;
            continue;
        }
        out.push(TranslatedBubble {
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
            original,
            reading,
            translated: t,
            shape: shapes.get(i).cloned().unwrap_or(None),
            needs_white_patch: false,
        });
    }
    if out.is_empty() && placeholder_skips > 0 {
        return Err(AppError::Network(
            "Model menggema format, bukan menerjemahkan. Coba lagi, atau ganti ke model vision yang lebih kuat.".into(),
        ));
    }
    Ok(PageTranslation {
        page_index: 0,
        bubbles: out,
        detected_lang: lang.to_string(),
        used_fallback: false,
    })
}

/// Petakan hasil full-image (persen → piksel asli, port `_mapFullImageResult`).
pub fn map_full_image_result(
    items: &[serde_json::Value],
    img_w: u32,
    img_h: u32,
    lang: &str,
) -> PageTranslation {
    let mut out = Vec::new();
    for item in items {
        let t = item
            .get("translated")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        if t.is_empty() || t.to_uppercase() == "SKIP" {
            continue;
        }
        let pct = |k: &str| item.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
        out.push(TranslatedBubble {
            x: (pct("x") / 100.0 * img_w as f64) as f32,
            y: (pct("y") / 100.0 * img_h as f64) as f32,
            w: (pct("w") / 100.0 * img_w as f64) as f32,
            h: (pct("h") / 100.0 * img_h as f64) as f32,
            original: String::new(),
            reading: String::new(),
            translated: t.to_string(),
            shape: None,
            needs_white_patch: false,
        });
    }
    PageTranslation {
        page_index: 0,
        bubbles: out,
        detected_lang: lang.to_string(),
        used_fallback: true,
    }
}

/// Ekstrak teks asisten dari respons OpenAI-compatible (string/part-list).
fn extract_openai_content(body: &serde_json::Value) -> String {
    let choices = body.get("choices").and_then(|c| c.as_array());
    let Some(first) = choices.and_then(|c| c.first()) else {
        return String::new();
    };
    match first.get("message").and_then(|m| m.get("content")) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// Ekstrak teks dari respons Gemini (`candidates[].content.parts[].text`).
fn extract_gemini_text(body: &serde_json::Value) -> String {
    body.get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

/// Ekstrak teks dari respons Cohere v2 (`message.content[].text`).
fn extract_cohere_text(body: &serde_json::Value) -> String {
    body.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn is_rate_limited(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
}

fn net_err(label: &str, e: impl std::fmt::Display) -> AppError {
    AppError::Network(format!("{label}: {e}"))
}

/// POST JSON + petakan 429 → `RateLimited` (kontrak round-robin mobile).
async fn post_json(
    http: &HttpClientManager,
    url: &str,
    headers: &[(&str, String)],
    payload: &serde_json::Value,
    label: &str,
) -> Result<serde_json::Value, AppError> {
    let mut req = http
        .client()
        .post(url)
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(120));
    for (k, v) in headers {
        req = req.header(*k, v.clone());
    }
    let res = req
        .json(payload)
        .send()
        .await
        .map_err(|e| net_err(label, e))?;
    if is_rate_limited(res.status()) {
        return Err(AppError::RateLimited(format!("{label} rate limited (429)")));
    }
    let res = res.error_for_status().map_err(|e| net_err(label, e))?;
    res.json::<serde_json::Value>()
        .await
        .map_err(|e| net_err(label, e))
}

/// Terjemahkan satu halaman via provider BYOK (mosaic ATAU full-image).
/// `boxes` kosong = fallback full-image (koordinat persen dari model).
/// `key` = kunci keychain (backend-only); `base_url_override` = kustom.
pub async fn translate_page(
    http: &HttpClientManager,
    provider: &AiProvider,
    key: &str,
    mosaic_jpeg: &[u8],
    img_w: u32,
    img_h: u32,
    boxes: &[MosaicBox],
    shapes: &[Option<Vec<Vec<i32>>>],
    target_lang: &str,
    style: TranslationStyle,
    skip_sfx: bool,
    reading_direction: &str,
    glossary: Option<&str>,
) -> Result<PageTranslation, AppError> {
    let b64 = B64.encode(mosaic_jpeg);
    match provider.kind {
        AiProviderKind::OpenAi | AiProviderKind::Custom => {
            let base = if provider.base_url.trim().is_empty() {
                provider.kind.default_base_url()
            } else {
                provider.base_url.trim()
            };
            if base.is_empty() {
                return Err(AppError::Validation(
                    "base URL wajib diisi untuk provider kustom".into(),
                ));
            }
            let url = format!("{}/chat/completions", base.trim_end_matches('/'));
            let prompt = if boxes.is_empty() {
                full_image_prompt(target_lang, style, skip_sfx)
            } else {
                mosaic_prompt(target_lang, style, skip_sfx, reading_direction, glossary)
            };
            let payload = serde_json::json!({
                "model": provider.model,
                "temperature": 0.3,
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{b64}")}},
                        {"type": "text", "text": prompt},
                    ],
                }],
            });
            let body = post_json(
                http,
                &url,
                &[("Authorization", format!("Bearer {key}"))],
                &payload,
                &provider.name,
            )
            .await?;
            let text = extract_openai_content(&body);
            if boxes.is_empty() {
                Ok(map_full_image_result(
                    &parse_json_array(&text)?,
                    img_w,
                    img_h,
                    target_lang,
                ))
            } else {
                map_mosaic_result(&parse_mosaic_json(&text)?, boxes, shapes, target_lang)
            }
        }
        AiProviderKind::Gemini => {
            let base = if provider.base_url.trim().is_empty() {
                provider.kind.default_base_url()
            } else {
                provider.base_url.trim()
            };
            let url = format!(
                "{}/{model}:generateContent?key={key}",
                base.trim_end_matches('/'),
                model = provider.model
            );
            let prompt = if boxes.is_empty() {
                full_image_prompt(target_lang, style, skip_sfx)
            } else {
                mosaic_prompt(target_lang, style, skip_sfx, reading_direction, glossary)
            };
            let payload = serde_json::json!({
                "contents": [{
                    "parts": [
                        {"inline_data": {"mime_type": "image/jpeg", "data": b64}},
                        {"text": prompt},
                    ],
                }],
                "generationConfig": {"temperature": 0.3},
            });
            let body = post_json(http, &url, &[], &payload, &provider.name).await?;
            let text = extract_gemini_text(&body);
            if boxes.is_empty() {
                Ok(map_full_image_result(
                    &parse_json_array(&text)?,
                    img_w,
                    img_h,
                    target_lang,
                ))
            } else {
                map_mosaic_result(&parse_mosaic_json(&text)?, boxes, shapes, target_lang)
            }
        }
        AiProviderKind::Cohere => {
            // Mobile pakai endpoint native v2 chat (bukan OpenAI-compatible).
            let url = "https://api.cohere.com/v2/chat";
            let prompt = if boxes.is_empty() {
                full_image_prompt(target_lang, style, skip_sfx)
            } else {
                mosaic_prompt(target_lang, style, skip_sfx, reading_direction, glossary)
            };
            let payload = serde_json::json!({
                "model": provider.model,
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": prompt},
                        {"type": "image_url", "image_url": {
                            "url": format!("data:image/jpeg;base64,{b64}"),
                            "detail": "auto",
                        }},
                    ],
                }],
            });
            let body = post_json(
                http,
                url,
                &[("Authorization", format!("Bearer {key}"))],
                &payload,
                &provider.name,
            )
            .await?;
            let text = extract_cohere_text(&body);
            if boxes.is_empty() {
                Ok(map_full_image_result(
                    &parse_json_array(&text)?,
                    img_w,
                    img_h,
                    target_lang,
                ))
            } else {
                map_mosaic_result(&parse_mosaic_json(&text)?, boxes, shapes, target_lang)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mosaic_prompt_memuat_id_style_sfx_glossary() {
        let p = mosaic_prompt(
            "id",
            TranslationStyle::Genz,
            true,
            "right-to-left",
            Some("Glossary:\n\"a\" -> \"b\""),
        );
        assert!(p.contains("red number ID"));
        assert!(p.contains("right-to-left"));
        assert!(p.contains("gue/lo"));
        assert!(p.contains("SKIP"));
        assert!(p.contains("Glossary:"));
        assert!(!mosaic_prompt(
            "id",
            TranslationStyle::Natural,
            false,
            "left-to-right",
            None
        )
        .contains("Glossary:"));
    }

    #[test]
    fn placeholder_echo_terdeteksi() {
        assert!(looks_like_placeholder("<terjemahan>"));
        assert!(looks_like_placeholder("..."));
        assert!(!looks_like_placeholder("Halo dunia!"));
        assert!(!looks_like_placeholder("Tunggu... sebentar"));
    }

    #[test]
    fn parse_mosaic_toleran_fence_dan_koma() {
        let raw = "ini hasil:\n```json\n{\"1\": {\"original\": \"おはよう\", \"reading\": \"ohayou\", \"translated\": \"selamat pagi\",}, \"2\": \"SKIP\",}\n```";
        let map = parse_mosaic_json(raw).unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn parse_mosaic_gagal_total_error_berguna() {
        assert!(parse_mosaic_json("tak ada json sama sekali").is_err());
    }

    #[test]
    fn petakan_mosaic_skip_dan_bentuk_baru() {
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(
            r#"{"1": {"original": "おはよう", "reading": "ohayou", "translated": "selamat pagi"}, "2": "SKIP", "3": "<terjemahan>"}"#,
        )
        .unwrap();
        let boxes = [
            MosaicBox {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 50.0,
            },
            MosaicBox {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            MosaicBox {
                x: 5.0,
                y: 5.0,
                w: 10.0,
                h: 10.0,
            },
        ];
        let res = map_mosaic_result(&map, &boxes, &[], "id").unwrap();
        assert_eq!(res.bubbles.len(), 1);
        assert_eq!(res.bubbles[0].translated, "selamat pagi");
        assert_eq!(res.bubbles[0].reading, "ohayou");
        assert!(!res.used_fallback);
    }

    #[test]
    fn petakan_full_image_persen_ke_piksel() {
        let items: Vec<serde_json::Value> =
            serde_json::from_str(r#"[{"x": 10, "y": 20, "w": 50, "h": 25, "translated": "halo"}]"#)
                .unwrap();
        let res = map_full_image_result(&items, 1000, 2000, "id");
        assert!(res.used_fallback);
        assert_eq!(res.bubbles[0].x, 100.0);
        assert_eq!(res.bubbles[0].h, 500.0);
    }
}
