//! image_ops — port `rust/src/image_ops.rs` (Fase 3).
//! chunk_webtoon / build_mosaic / compress_page jadi `pub fn` biasa +
//! `#[tauri::command]` wrapper (tanpa `with_image_ops_lock` JNI).

use crate::core::AppError;

#[allow(dead_code)]
pub fn chunk_webtoon(_bytes: &[u8], _max_chunk_h: u32) -> Result<Vec<Vec<u8>>, AppError> {
    Err(AppError::Internal(
        "Fase 3: image_ops belum di-port".to_string(),
    ))
}
