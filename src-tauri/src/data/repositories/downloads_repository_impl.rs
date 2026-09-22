//! DownloadsRepositoryImpl — baris SQLite `downloads` (8.1).
//! state_json = serde `DownloadState`; source_id kolom terpisah.

use std::path::Path;

use async_trait::async_trait;

use crate::{
    core::AppError,
    data::datasources::local::SqliteDs,
    domain::{repositories::DownloadsRepository, DownloadState, DownloadTask},
};

pub struct DownloadsRepositoryImpl {
    db: SqliteDs,
}

impl DownloadsRepositoryImpl {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            db: SqliteDs::open(path)?,
        })
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        Ok(Self {
            db: SqliteDs::open_in_memory()?,
        })
    }

    fn row_to_task(
        chapter_id: String,
        content_id: String,
        source_id: String,
        state_json: String,
    ) -> Result<DownloadTask, AppError> {
        let state: DownloadState = serde_json::from_str(&state_json)
            .map_err(|e| AppError::Storage(format!("state_json download rusak: {e}")))?;
        Ok(DownloadTask {
            chapter_id,
            content_id,
            source_id,
            state,
        })
    }
}

#[async_trait]
impl DownloadsRepository for DownloadsRepositoryImpl {
    async fn save(&self, task: &DownloadTask) -> Result<(), AppError> {
        let state_json = serde_json::to_string(&task.state)
            .map_err(|e| AppError::Internal(format!("serialize state: {e}")))?;
        self.db.save_download(
            &task.chapter_id,
            &task.content_id,
            &task.source_id,
            &state_json,
        )
    }

    async fn get(&self, chapter_id: &str) -> Result<Option<DownloadTask>, AppError> {
        match self.db.get_download_row(chapter_id)? {
            None => Ok(None),
            Some((content_id, source_id, state_json)) => {
                Self::row_to_task(chapter_id.to_string(), content_id, source_id, state_json)
                    .map(Some)
            }
        }
    }

    async fn list(&self) -> Result<Vec<DownloadTask>, AppError> {
        self.db
            .list_download_rows()?
            .into_iter()
            .map(|(chapter_id, content_id, source_id, state_json)| {
                Self::row_to_task(chapter_id, content_id, source_id, state_json)
            })
            .collect()
    }

    async fn delete(&self, chapter_id: &str) -> Result<bool, AppError> {
        self.db.delete_download(chapter_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(chapter: &str, state: DownloadState) -> DownloadTask {
        DownloadTask {
            chapter_id: chapter.into(),
            content_id: "m1".into(),
            source_id: "nhentai".into(),
            state,
        }
    }

    #[test]
    fn download_roundtrip_save_get_list_delete() {
        let repo = DownloadsRepositoryImpl::open_in_memory().unwrap();
        tauri::async_runtime::block_on(async {
            assert!(repo.get("c1").await.unwrap().is_none());
            repo.save(&task(
                "c1",
                DownloadState::Downloading { page: 2, total: 10 },
            ))
            .await
            .unwrap();
            let got = repo.get("c1").await.unwrap().unwrap();
            assert_eq!(got.source_id, "nhentai");
            assert_eq!(
                serde_json::to_value(&got.state).unwrap(),
                serde_json::json!({"Downloading": {"page": 2, "total": 10}})
            );
            // Upsert state.
            repo.save(&task("c1", DownloadState::Completed))
                .await
                .unwrap();
            let got = repo.get("c1").await.unwrap().unwrap();
            assert_eq!(
                serde_json::to_value(&got.state).unwrap(),
                serde_json::json!("Completed")
            );
            repo.save(&task("c2", DownloadState::Paused)).await.unwrap();
            let all = repo.list().await.unwrap();
            assert_eq!(all.len(), 2);
            assert!(repo.delete("c1").await.unwrap());
            assert!(!repo.delete("c1").await.unwrap());
            assert_eq!(repo.list().await.unwrap().len(), 1);
        });
    }

    #[test]
    fn corrupt_state_json_is_storage_error() {
        let repo = DownloadsRepositoryImpl::open_in_memory().unwrap();
        repo.db
            .save_download("cX", "m1", "nhentai", "not-json")
            .unwrap();
        tauri::async_runtime::block_on(async {
            let err = repo.get("cX").await.unwrap_err();
            assert!(err.to_string().contains("rusak"), "{err}");
        });
    }
}
