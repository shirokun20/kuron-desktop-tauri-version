//! AI provider BYOK commands (9.2). Thin: validasi di use case,
//! `AppError -> String` di boundary. Kunci API TIDAK PERNAH dibalikkan
//! ke frontend — hanya `has_key`. Pengecualian `cmd_ai_model_catalog`:
//! kunci yang dikirim UI (belum tersimpan) hanya dipakai sekali untuk
//! GET daftar model, tak disimpan dan tak dibalikkan.

use tauri::State;

use tauri::Emitter;

use crate::{
    application::{
        DeleteAiProviderUseCase, ListAiProvidersUseCase, SaveAiProviderUseCase, TranslatePageInput,
        TranslatePageUseCase,
    },
    core::AppState,
    data::datasources::ai_model_catalog,
    domain::{
        AiModelOption, AiProvider, AiProviderInput, AiProviderKind, BubbleBox, PageTranslation,
        TranslationStyle,
    },
};

#[tauri::command]
pub async fn cmd_ai_providers_list(state: State<'_, AppState>) -> Result<Vec<AiProvider>, String> {
    ListAiProvidersUseCase::new(state.ai_providers.clone())
        .execute()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_ai_provider_save(
    state: State<'_, AppState>,
    provider: AiProviderInput,
    api_key: String,
) -> Result<AiProvider, String> {
    SaveAiProviderUseCase::new(state.ai_providers.clone())
        .execute(provider, &api_key)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_ai_provider_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    DeleteAiProviderUseCase::new(state.ai_providers.clone())
        .execute(&id)
        .await
        .map_err(|e| e.to_string())
}

/// LOV model live dari endpoint `GET /models` milik provider — port
/// `AiModelCatalogRepository` mobile. `api_key` = kunci yang sedang diketik
/// di form (baca-saja, tak disimpan); `Custom` ditolak (input manual).
#[tauri::command]
pub async fn cmd_ai_model_catalog(
    state: State<'_, AppState>,
    kind: AiProviderKind,
    api_key: String,
) -> Result<Vec<AiModelOption>, String> {
    ai_model_catalog::fetch_models(&state.http, kind, &api_key)
        .await
        .map_err(|e| e.to_string())
}

/// Input translate satu halaman dari FE.
/// Backend yang mengambil bytes (`page_url` remote via HTTP dengan header
/// sumber, lokal via baca file) — FE cukup kirim URL, tanpa fetch ganda
/// (hindari CORS WebView) dan tanpa payload base64 raksasa di IPC.
#[derive(Debug, serde::Deserialize)]
pub struct TranslatePageArgs {
    /// URL remote (`http…`) atau path file lokal gambar halaman.
    pub page_url: String,
    /// ID sumber (header UA/referer hotlink-guard).
    pub source_id: String,
    pub content_id: String,
    pub page_index: u32,
    pub crop_y_top: Option<u32>,
    /// `true` = manga RTL; `None` = ikut pengaturan reader.
    pub rtl: Option<bool>,
    pub style: Option<TranslationStyle>,
    pub skip_sfx: Option<bool>,
    pub target_lang: Option<String>,
    /// Bubble deteksi yang sudah ada (draw/prefetch reuse).
    pub pre_detected: Option<Vec<BubbleBox>>,
    /// Bubble manual user (pengganti otoritas).
    pub manual: Option<Vec<BubbleBox>>,
}

/// Terjemahkan satu halaman: detect → mosaic → AI BYOK → cache (7.2).
/// Progress via event `ai:progress` (`detecting|mosaic|translating|done`).
/// Kunci API dibaca dari keychain di backend — tak pernah lewat FE.
#[tauri::command]
pub async fn cmd_translate_page(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    args: TranslatePageArgs,
) -> Result<PageTranslation, String> {
    let emit = |stage: &str| {
        let _ = app.emit(
            "ai:progress",
            serde_json::json!({
                "stage": stage,
                "contentId": args.content_id,
                "pageIndex": args.page_index,
            }),
        );
    };
    // Ambil bytes halaman: remote → HTTP + header sumber; lokal → file.
    let raw = if args.page_url.starts_with("http://") || args.page_url.starts_with("https://") {
        state
            .http
            .get_bytes(&args.page_url, &args.source_id, None)
            .await
            .map_err(|e| e.to_string())?
    } else {
        std::fs::read(&args.page_url).map_err(|e| format!("baca file halaman: {e}"))?
    };
    let rtl = args.rtl.unwrap_or(state_rtl_default());
    emit("detecting");
    // Model 42MB diunduh sekali ke data-dir (keputusan user) sebelum deteksi.
    state
        .bubble_detector
        .ensure_model(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    let usecase = TranslatePageUseCase::new(
        state.ai_providers.clone(),
        state.translation_cache.clone(),
        state.bubble_detector.clone(),
        state.http.clone(),
    );
    emit("translating");
    let out = usecase
        .execute(TranslatePageInput {
            image: raw,
            content_id: args.content_id.clone(),
            page_index: args.page_index,
            image_url: args.page_url.clone(),
            crop_y_top: args.crop_y_top.unwrap_or(0),
            rtl,
            style: args.style.unwrap_or(TranslationStyle::Natural),
            skip_sfx: args.skip_sfx.unwrap_or(true),
            target_lang: args
                .target_lang
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "id".to_string()),
            pre_detected: args.pre_detected.clone().unwrap_or_default(),
            manual: args.manual.clone().unwrap_or_default(),
            glossary: Vec::new(),
        })
        .await
        .map_err(|e| e.to_string())?;
    emit("done");
    Ok(out)
}

/// Default arah baca: pengaturan reader global tak bisa dibaca sinkron di
/// command — FE mengirim `rtl` eksplisit; fallback aman = manga RTL.
fn state_rtl_default() -> bool {
    true
}
