//! Download commands (8.1). Thin: resolve repo sumber → usecase/manager,
//! `AppError -> String` di boundary. Event progress/completed dari manager
//! (sink Tauri dipasang di `lib.rs` setup).

use tauri::{AppHandle, Emitter, State};

use crate::{
    application::{
        DownloadEventSink, ListDownloadsUseCase, RemoveDownloadUseCase, StartDownloadUseCase,
    },
    core::AppState,
    domain::DownloadTask,
};

/// Sink produksi: emit ke semua jendela (`download:progress` / `download:completed`).
pub struct TauriDownloadSink {
    pub app: AppHandle,
}

impl DownloadEventSink for TauriDownloadSink {
    fn progress(&self, task: &DownloadTask) {
        let _ = self.app.emit("download:progress", task);
    }

    fn completed(&self, task: &DownloadTask) {
        let _ = self.app.emit("download:completed", task);
    }
}

#[tauri::command]
pub async fn cmd_download_start(
    state: State<'_, AppState>,
    chapter_id: String,
    content_id: String,
    source_id: String,
    total: Option<u32>,
) -> Result<DownloadTask, String> {
    let repo = state
        .repo_for(&source_id.to_lowercase())
        .map_err(|e| e.to_string())?;
    StartDownloadUseCase::new(repo, state.download_manager.clone())
        .execute(&chapter_id, &content_id, &source_id, total)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_download_pause(
    state: State<'_, AppState>,
    chapter_id: String,
) -> Result<DownloadTask, String> {
    state.download_manager.pause(&chapter_id);
    // Tunggu loop melihat flag (brief); snapshot biasanya sudah Paused.
    for _ in 0..40 {
        if let Some(t) = state.download_manager.snapshot(&chapter_id) {
            if matches!(t.state, crate::domain::DownloadState::Paused) {
                return Ok(t);
            }
        }
        if !state.download_manager.is_running(&chapter_id) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    state
        .download_manager
        .snapshot(&chapter_id)
        .or(state
            .downloads
            .get(&chapter_id)
            .await
            .map_err(|e| e.to_string())?)
        .ok_or_else(|| format!("unduhan {chapter_id} tidak ditemukan"))
}

#[tauri::command]
pub async fn cmd_download_resume(
    state: State<'_, AppState>,
    chapter_id: String,
) -> Result<DownloadTask, String> {
    // Ambil metadata dari snapshot/DB → start ulang (cache di-skip).
    let task = state
        .download_manager
        .snapshot(&chapter_id)
        .or(state
            .downloads
            .get(&chapter_id)
            .await
            .map_err(|e| e.to_string())?)
        .ok_or_else(|| format!("unduhan {chapter_id} tidak ditemukan"))?;
    if matches!(task.state, crate::domain::DownloadState::Completed) {
        return Ok(task);
    }
    let repo = state
        .repo_for(&task.source_id.to_lowercase())
        .map_err(|e| e.to_string())?;
    let pages = {
        use crate::domain::repositories::ContentRepository;
        repo.get_page_images(&chapter_id)
            .await
            .map_err(|e| e.to_string())?
    };
    let url_count = pages
        .iter()
        .filter(|p| matches!(p, crate::domain::PageImageResult::Remote(_)))
        .count() as u32;
    state
        .download_manager
        .start(
            &chapter_id,
            &task.content_id,
            &task.source_id,
            pages,
            url_count,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_download_list(state: State<'_, AppState>) -> Result<Vec<DownloadTask>, String> {
    ListDownloadsUseCase::new(state.downloads.clone())
        .execute()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_download_status(
    state: State<'_, AppState>,
    chapter_id: String,
) -> Result<Option<DownloadTask>, String> {
    if let Some(t) = state.download_manager.snapshot(&chapter_id) {
        return Ok(Some(t));
    }
    state
        .downloads
        .get(&chapter_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_download_remove(
    state: State<'_, AppState>,
    chapter_id: String,
) -> Result<bool, String> {
    RemoveDownloadUseCase::new(state.download_manager.clone())
        .execute(&chapter_id)
        .await
        .map_err(|e| e.to_string())
}
