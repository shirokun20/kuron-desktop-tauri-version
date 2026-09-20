//! Content commands: feed halaman utama (mock Fase 0, real Fase 2).
//! Repo di-resolve dari `AppState` (DI), bukan dikonstruksi di sini.

use tauri::State;

use crate::{application::GetHomeFeedUseCase, core::AppState, domain::Content};

#[tauri::command]
pub async fn cmd_home_feed(state: State<'_, AppState>) -> Result<Vec<Content>, String> {
    GetHomeFeedUseCase::new(state.content_repo.clone())
        .execute()
        .await
        .map_err(|e| e.to_string())
}
