//! LibraryRepositoryImpl — riwayat/favorit di atas `SqliteDs` (8.3).
//! Murni lokal SQLite: tanpa request keluar (privasi, syarat 8.3).

use std::path::Path;

use async_trait::async_trait;

use crate::{
    core::AppError,
    data::datasources::local::SqliteDs,
    domain::{repositories::LibraryRepository, Content, HistoryItem},
};

pub struct LibraryRepositoryImpl {
    db: SqliteDs,
}

impl LibraryRepositoryImpl {
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
}

#[async_trait]
impl LibraryRepository for LibraryRepositoryImpl {
    async fn record_history(&self, content: &Content, position: i64) -> Result<(), AppError> {
        self.db.record_history(content, position)
    }

    async fn list_history(&self, limit: i64) -> Result<Vec<HistoryItem>, AppError> {
        Ok(self
            .db
            .list_history_contents(limit)?
            .into_iter()
            .map(|(c, position, updated_at)| HistoryItem {
                content_id: c.id,
                title: c.title,
                cover_url: c.cover_url,
                source_id: c.source_id,
                position,
                updated_at,
            })
            .collect())
    }

    async fn remove_history(&self, content_id: &str) -> Result<(), AppError> {
        self.db.remove_history(content_id)
    }

    async fn clear_history(&self) -> Result<(), AppError> {
        self.db.clear_history()
    }

    async fn set_favorite(&self, content: &Content, fav: bool) -> Result<(), AppError> {
        // Snapshot display data dulu (ala `addToFavorites` mobile), lalu flag.
        // Unfav menyimpan snapshot terbaru juga — daftar tetap konsisten.
        let mut snap = content.clone();
        snap.is_favorite = fav;
        self.db.save_content(&snap)?;
        self.db.set_favorite(&content.id, fav)
    }

    async fn list_favorites(&self) -> Result<Vec<Content>, AppError> {
        self.db.list_favorite_contents()
    }

    async fn clear_library(&self) -> Result<(), AppError> {
        self.db.clear_library()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(id: &str) -> Content {
        Content {
            id: id.to_string(),
            title: format!("Judul {id}"),
            cover_url: format!("https://cdn.example/{id}.jpg"),
            source_id: "nhentai".to_string(),
            upload_date: None,
            is_favorite: false,
            page_count: None,
            language: None,
            tags: Vec::new(),
            available_languages: Vec::new(),
            description: None,
            rating: None,
            favorites: None,
        }
    }

    #[test]
    fn favorite_roundtrip_keeps_snapshot_and_order() {
        let repo = LibraryRepositoryImpl::open_in_memory().unwrap();
        assert!(tauri::async_runtime::block_on(repo.list_favorites())
            .unwrap()
            .is_empty());
        tauri::async_runtime::block_on(repo.set_favorite(&content("m1"), true)).unwrap();
        tauri::async_runtime::block_on(repo.set_favorite(&content("m2"), true)).unwrap();
        let favs = tauri::async_runtime::block_on(repo.list_favorites()).unwrap();
        assert_eq!(favs.len(), 2);
        assert!(favs.iter().all(|c| c.is_favorite));
        assert_eq!(favs[0].title, "Judul m2");
        assert_eq!(favs[0].cover_url, "https://cdn.example/m2.jpg");
        tauri::async_runtime::block_on(repo.set_favorite(&content("m2"), false)).unwrap();
        let favs = tauri::async_runtime::block_on(repo.list_favorites()).unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id, "m1");
    }

    #[test]
    fn history_records_latest_first_and_clears() {
        let repo = LibraryRepositoryImpl::open_in_memory().unwrap();
        tauri::async_runtime::block_on(repo.record_history(&content("m1"), 5)).unwrap();
        tauri::async_runtime::block_on(repo.record_history(&content("m2"), 1)).unwrap();
        // Upsert: posisi + urutan terbaru diperbarui.
        tauri::async_runtime::block_on(repo.record_history(&content("m1"), 9)).unwrap();
        let hist = tauri::async_runtime::block_on(repo.list_history(10)).unwrap();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].content_id, "m1");
        assert_eq!(hist[0].title, "Judul m1");
        assert_eq!(hist[0].position, 9);
        assert!(hist[0].updated_at > 0);
        // Hapus per-item: m2 hilang, m1 utuh.
        tauri::async_runtime::block_on(repo.remove_history("m2")).unwrap();
        let hist = tauri::async_runtime::block_on(repo.list_history(10)).unwrap();
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].content_id, "m1");
        tauri::async_runtime::block_on(repo.clear_history()).unwrap();
        assert!(tauri::async_runtime::block_on(repo.list_history(10))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn clear_library_wipes_history_favorites_snapshots() {
        let repo = LibraryRepositoryImpl::open_in_memory().unwrap();
        tauri::async_runtime::block_on(repo.record_history(&content("m1"), 3)).unwrap();
        tauri::async_runtime::block_on(repo.set_favorite(&content("m1"), true)).unwrap();
        tauri::async_runtime::block_on(repo.clear_library()).unwrap();
        assert!(tauri::async_runtime::block_on(repo.list_history(10))
            .unwrap()
            .is_empty());
        assert!(tauri::async_runtime::block_on(repo.list_favorites())
            .unwrap()
            .is_empty());
    }
}
