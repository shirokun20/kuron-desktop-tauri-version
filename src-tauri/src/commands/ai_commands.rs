//! AI provider BYOK commands (9.2). Thin: validasi di use case,
//! `AppError -> String` di boundary. Kunci API TIDAK PERNAH dibalikkan
//! ke frontend — hanya `has_key`.

use tauri::State;

use crate::{
    application::{DeleteAiProviderUseCase, ListAiProvidersUseCase, SaveAiProviderUseCase},
    core::AppState,
    domain::{AiProvider, AiProviderInput},
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
