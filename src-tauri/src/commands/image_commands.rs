//! Image ops commands (5.1). Thin wrapper + `spawn_blocking` (design #7):
//! kerja berat `image_ops` tidak boleh memblokir event loop Tauri.
//! Kontrak FE (`api.image*`): payload base64 → hasil base64 (string / string[]).
//! Murni transform bytes — tanpa business logic di command.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::core::AppState;
use crate::data::native::bubble_detector::post_process_boxes;
use crate::data::native::image_ops;
use crate::domain::BubbleBox;

fn blocking<F, T>(f: F) -> impl std::future::Future<Output = Result<T, String>>
where
    F: FnOnce() -> Result<T, crate::core::AppError> + Send + 'static,
    T: Send + 'static,
{
    async move {
        tauri::async_runtime::spawn_blocking(f)
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())
    }
}

fn decode_b64(data: &str) -> Result<Vec<u8>, String> {
    B64.decode(data).map_err(|e| format!("base64 invalid: {e}"))
}

fn encode_b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

/// Potong gambar webtoon tinggi menjadi chunk JPEG ≤ `max_chunk_h` (quality 90).
#[tauri::command]
pub async fn cmd_image_chunk_webtoon(
    data: String,
    max_chunk_h: u32,
) -> Result<Vec<String>, String> {
    let raw = decode_b64(&data)?;
    let chunks = blocking(move || image_ops::chunk_webtoon(&raw, max_chunk_h)).await?;
    Ok(chunks.iter().map(|c| encode_b64(c)).collect())
}

/// Susun mosaic dari daftar bubble flat `[x,y,w,h, …]` — JPEG < 2MB, base64.
#[tauri::command]
pub async fn cmd_image_build_mosaic(data: String, boxes: Vec<u32>) -> Result<String, String> {
    if boxes.len() % 4 != 0 {
        return Err(format!(
            "boxes harus kelipatan 4 (x,y,w,h); panjang {}",
            boxes.len()
        ));
    }
    let rects: Vec<(u32, u32, u32, u32)> = boxes
        .chunks_exact(4)
        .map(|c| (c[0], c[1], c[2], c[3]))
        .collect();
    let raw = decode_b64(&data)?;
    let jpeg = blocking(move || image_ops::build_mosaic(&raw, &rects)).await?;
    Ok(encode_b64(&jpeg))
}

/// Kompres halaman penuh: sisi terpanjang ≤ `max_dim`, JPEG 85, base64.
#[tauri::command]
pub async fn cmd_image_compress_page(data: String, max_dim: u32) -> Result<String, String> {
    let raw = decode_b64(&data)?;
    let out = blocking(move || image_ops::compress_page(&raw, max_dim)).await?;
    Ok(encode_b64(&out))
}

/// Status model bubble: sudah terunduh atau belum + path lokal.
/// Model 42MB diunduh sekali (bukan bundel); deteksi tanpa model = error jelas.
#[tauri::command]
pub fn cmd_bubble_model_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let det = state.bubble_detector.clone();
    Ok(serde_json::json!({
        "ready": det.model_path().exists(),
        "path": det.model_path().to_string_lossy(),
    }))
}

/// Deteksi bubble manga (YOLO-seg `ort` CPU + NMS 0.45, port `BubbleDetector.kt`).
/// Backend yang mengambil bytes (`page_url` remote via HTTP + header sumber,
/// lokal via baca file) — FE cukup kirim URL, tanpa fetch ganda (hindari
/// CORS WebView) dan tanpa payload base64 raksasa di IPC.
/// Model diunduh otomatis saat pertama dipakai; inferensi via `spawn_blocking`.
#[tauri::command]
pub async fn cmd_detect_bubbles(
    state: tauri::State<'_, AppState>,
    page_url: String,
    source_id: String,
) -> Result<Vec<BubbleBox>, String> {
    let raw = if page_url.starts_with("http://") || page_url.starts_with("https://") {
        state
            .http
            .get_bytes(&page_url, &source_id, None)
            .await
            .map_err(|e| e.to_string())?
    } else {
        std::fs::read(&page_url).map_err(|e| format!("baca file halaman: {e}"))?
    };
    let det = state.bubble_detector.clone();
    let http = state.http.clone();
    blocking(move || {
        tauri::async_runtime::block_on(det.ensure_model(&http))?;
        let boxes = det.detect(&raw)?;
        Ok(post_process_boxes(boxes))
    })
    .await
}
