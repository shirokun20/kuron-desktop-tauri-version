//! Content commands: feed halaman utama (mock Fase 0, real Fase 2).
//! Search/detail/chapters/pages: kontrak baru Fase 2, mock kini.
//! Repo di-resolve dari `AppState` (DI), bukan dikonstruksi di sini.

use tauri::State;

use crate::{
    application::{
        GetChaptersUseCase, GetContentDetailUseCase, GetHomeFeedUseCase,
        GetPageImagesUseCase, SearchContentUseCase,
    },
    core::AppState,
    domain::{Chapter, Content, PageImageResult, SearchFilter},
};

#[tauri::command]
pub async fn cmd_home_feed(
    state: State<'_, AppState>,
    source: Option<String>,
    page: Option<u32>,
) -> Result<Vec<Content>, String> {
    let page = page.unwrap_or(1).max(1);
    let repo = match source.as_deref() {
        None | Some("semua") => state.content_repo.clone(),
        Some(id) => state.repo_for(&id.to_lowercase()).map_err(|e| e.to_string())?,
    };
    GetHomeFeedUseCase::new(repo).execute(page).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_search(
    state: State<'_, AppState>,
    filter: SearchFilter,
) -> Result<Vec<Content>, String> {
    SearchContentUseCase::new(state.content_repo.clone())
        .execute(filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_detail(
    state: State<'_, AppState>,
    content_id: String,
) -> Result<Content, String> {
    GetContentDetailUseCase::new(state.content_repo.clone())
        .execute(&content_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_chapters(
    state: State<'_, AppState>,
    content_id: String,
) -> Result<Vec<Chapter>, String> {
    GetChaptersUseCase::new(state.content_repo.clone())
        .execute(&content_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_page_images(
    state: State<'_, AppState>,
    chapter_id: String,
) -> Result<Vec<PageImageResult>, String> {
    GetPageImagesUseCase::new(state.content_repo.clone())
        .execute(&chapter_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MockContentRepository;

    #[test]
    fn new_usecases_serve_mock() {
        let detail = tauri::async_runtime::block_on(
            GetContentDetailUseCase::new(MockContentRepository).execute("m1"),
        )
        .unwrap();
        assert!(detail.title.contains("m1"));
        let found = tauri::async_runtime::block_on(
            SearchContentUseCase::new(MockContentRepository).execute(SearchFilter {
                query: "x".into(),
                source_id: None,
                page: 1,
            }),
        )
        .unwrap();
        assert_eq!(found.len(), 8);
        assert!(
            tauri::async_runtime::block_on(
                GetChaptersUseCase::new(MockContentRepository).execute("m1")
            )
            .unwrap()
            .is_empty()
        );
        assert!(
            tauri::async_runtime::block_on(
                GetPageImagesUseCase::new(MockContentRepository).execute("c1")
            )
            .unwrap()
            .is_empty()
        );
    }
}
