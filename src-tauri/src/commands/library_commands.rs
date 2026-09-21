//! Library commands: riwayat + favorit + reset (8.3). Thin: no logic,
//! `AppError -> String` di boundary. Repo dari `AppState` (DI).

use tauri::State;

use crate::{
    application::{
        ClearHistoryUseCase, ClearLibraryUseCase, ListFavoritesUseCase, ListHistoryUseCase,
        RecordHistoryUseCase, RemoveHistoryUseCase, SetFavoriteUseCase,
    },
    core::AppState,
    domain::{Content, HistoryItem},
};

#[tauri::command]
pub async fn cmd_history_record(
    state: State<'_, AppState>,
    content: Content,
    position: i64,
) -> Result<(), String> {
    RecordHistoryUseCase::new(state.library.clone())
        .execute(&content, position)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_history_list(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<HistoryItem>, String> {
    ListHistoryUseCase::new(state.library.clone())
        .execute(limit.unwrap_or(50).max(1))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_history_remove(
    state: State<'_, AppState>,
    content_id: String,
) -> Result<(), String> {
    RemoveHistoryUseCase::new(state.library.clone())
        .execute(&content_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_history_clear(state: State<'_, AppState>) -> Result<(), String> {
    ClearHistoryUseCase::new(state.library.clone())
        .execute()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_favorite_set(
    state: State<'_, AppState>,
    content: Content,
    fav: bool,
) -> Result<(), String> {
    SetFavoriteUseCase::new(state.library.clone())
        .execute(&content, fav)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_favorite_list(state: State<'_, AppState>) -> Result<Vec<Content>, String> {
    ListFavoritesUseCase::new(state.library.clone())
        .execute()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_library_clear(state: State<'_, AppState>) -> Result<(), String> {
    ClearLibraryUseCase::new(state.library.clone())
        .execute()
        .await
        .map_err(|e| e.to_string())
}
