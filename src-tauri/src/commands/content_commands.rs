//! Content commands: feed halaman utama (mock Fase 0, real Fase 2).

use crate::{
    application::GetHomeFeedUseCase, data::MockContentRepository, domain::Content,
};

#[tauri::command]
pub fn cmd_home_feed() -> Result<Vec<Content>, String> {
    GetHomeFeedUseCase::new(MockContentRepository)
        .execute()
        .map_err(|e| e.to_string())
}
