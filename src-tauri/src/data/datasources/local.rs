//! Local DataSources — ganti sqflite/shared_preferences/path_provider (Fase 2).
//! Backend-owned: `rusqlite` (bundled) agar CRUD teruji via `cargo test`
//! tanpa konteks Tauri; frontend tetap lewat `invoke('cmd_*')` (layer rule).
//! `tauri-plugin-sql` ditolak untuk backend: akses SQL langsung dari frontend
//! akan by-pass lapisan Application — lihat ringkasan sesi 4.x di MEMORY.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{core::AppError, domain::{Chapter, Content}};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS contents (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  cover_url TEXT NOT NULL DEFAULT '',
  source_id TEXT NOT NULL DEFAULT '',
  upload_date TEXT,
  is_favorite INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS chapters (
  id TEXT PRIMARY KEY,
  content_id TEXT NOT NULL,
  title TEXT NOT NULL DEFAULT '',
  ch_order INTEGER NOT NULL DEFAULT 0,
  is_external INTEGER NOT NULL DEFAULT 0,
  external_url TEXT
);
CREATE TABLE IF NOT EXISTS downloads (
  chapter_id TEXT PRIMARY KEY,
  content_id TEXT NOT NULL,
  state_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS translation_cache (
  key TEXT PRIMARY KEY,
  data BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS history (
  content_id TEXT PRIMARY KEY,
  position INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS favorites (
  content_id TEXT PRIMARY KEY,
  added_at INTEGER NOT NULL DEFAULT 0
);
";

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// SQLite lokal (`kuron.db`): contents, chapters, downloads,
/// translation_cache, history, favorites.
pub struct SqliteDs {
    conn: Mutex<Connection>,
}

impl SqliteDs {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let ds = Self {
            conn: Mutex::new(conn),
        };
        ds.migrate()?;
        Ok(ds)
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()?;
        let ds = Self {
            conn: Mutex::new(conn),
        };
        ds.migrate()?;
        Ok(ds)
    }

    fn migrate(&self) -> Result<(), AppError> {
        self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?.execute_batch(SCHEMA)?;
        Ok(())
    }

    pub fn save_content(&self, c: &Content) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.execute(
            "INSERT INTO contents (id, title, cover_url, source_id, upload_date, is_favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title, cover_url=excluded.cover_url,
               source_id=excluded.source_id, upload_date=excluded.upload_date,
               is_favorite=excluded.is_favorite",
            params![
                c.id,
                c.title,
                c.cover_url,
                c.source_id,
                c.upload_date,
                c.is_favorite as i32
            ],
        )?;
        Ok(())
    }

    pub fn get_content(&self, id: &str) -> Result<Option<Content>, AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.query_row(
            "SELECT id, title, cover_url, source_id, upload_date, is_favorite
             FROM contents WHERE id = ?1",
            params![id],
            |row| {
                Ok(Content {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    cover_url: row.get(2)?,
                    source_id: row.get(3)?,
                    upload_date: row.get(4)?,
                    is_favorite: row.get::<_, i32>(5)? != 0,
                    page_count: None,
                    language: None,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
    }

    pub fn save_chapters(&self, chapters: &[Chapter]) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        for ch in chapters {
            conn.execute(
                "INSERT INTO chapters (id, content_id, title, ch_order, is_external, external_url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET title=excluded.title, ch_order=excluded.ch_order,
                   is_external=excluded.is_external, external_url=excluded.external_url",
                params![
                    ch.id,
                    ch.content_id,
                    ch.title,
                    ch.order as i64,
                    ch.is_external as i32,
                    ch.external_url
                ],
            )?;
        }
        Ok(())
    }

    pub fn record_history(&self, content_id: &str, position: i64) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.execute(
            "INSERT INTO history (content_id, position, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(content_id) DO UPDATE SET position=excluded.position,
               updated_at=excluded.updated_at",
            params![content_id, position, now_secs()],
        )?;
        Ok(())
    }

    pub fn list_history(&self, limit: i64) -> Result<Vec<(String, i64)>, AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        let mut stmt = conn.prepare(
            "SELECT content_id, position FROM history ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn clear_history(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn set_favorite(&self, content_id: &str, fav: bool) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        if fav {
            conn.execute(
                "INSERT INTO favorites (content_id, added_at) VALUES (?1, ?2)
                 ON CONFLICT(content_id) DO NOTHING",
                params![content_id, now_secs()],
            )?;
        } else {
            conn.execute("DELETE FROM favorites WHERE content_id = ?1", params![content_id])?;
        }
        Ok(())
    }

    pub fn list_favorites(&self) -> Result<Vec<String>, AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        let mut stmt = conn.prepare("SELECT content_id FROM favorites ORDER BY added_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn save_download(
        &self,
        chapter_id: &str,
        content_id: &str,
        state_json: &str,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.execute(
            "INSERT INTO downloads (chapter_id, content_id, state_json) VALUES (?1, ?2, ?3)
             ON CONFLICT(chapter_id) DO UPDATE SET state_json=excluded.state_json",
            params![chapter_id, content_id, state_json],
        )?;
        Ok(())
    }

    pub fn get_download(&self, chapter_id: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.query_row(
            "SELECT state_json FROM downloads WHERE chapter_id = ?1",
            params![chapter_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(AppError::from)
    }

    pub fn put_translation(&self, key: &str, data: &[u8]) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.execute(
            "INSERT INTO translation_cache (key, data) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET data=excluded.data",
            params![key, data],
        )?;
        Ok(())
    }

    pub fn get_translation(&self, key: &str) -> Result<Option<Vec<u8>>, AppError> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Storage(format!("sqlite lock: {e}"))
        })?;
        conn.query_row(
            "SELECT data FROM translation_cache WHERE key = ?1",
            params![key],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(AppError::from)
    }
}

/// KV file-backed (pengaturan, pilihan UI backend) — ganti shared_preferences
/// untuk sisi backend. Frontend tetap localStorage untuk tema/sumber.
pub struct KvStoreDs {
    path: PathBuf,
    map: Mutex<HashMap<String, String>>,
}

impl KvStoreDs {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let map = if path.exists() {
            let raw = std::fs::read_to_string(path)?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Ok(Self {
            path: path.to_path_buf(),
            map: Mutex::new(map),
        })
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut map = self.map.lock().map_err(|e| {
            AppError::Storage(format!("kv lock: {e}"))
        })?;
        map.insert(key.to_string(), value.to_string());
        std::fs::write(&self.path, serde_json::to_string(&*map).unwrap())?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.map.lock().ok()?.get(key).cloned()
    }
}

/// Secret di keychain OS — ganti flutter_secure_storage (kunci API AI, cookie CF).
/// TIDAK pernah di file plaintext / log.
pub struct SecretStore {
    service: String,
}

impl SecretStore {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    fn entry(&self, account: &str) -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(&self.service, account)
            .map_err(|e| AppError::Storage(format!("keychain: {e}")))
    }

    pub fn set_secret(&self, account: &str, secret: &str) -> Result<(), AppError> {
        self.entry(account)?
            .set_password(secret)
            .map_err(|e| AppError::Storage(format!("keychain: {e}")))
    }

    pub fn get_secret(&self, account: &str) -> Result<Option<String>, AppError> {
        match self.entry(account)?.get_password() {
            Ok(pw) => Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::Storage(format!("keychain: {e}"))),
        }
    }

    pub fn delete_secret(&self, account: &str) -> Result<(), AppError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Storage(format!("keychain: {e}"))),
        }
    }
}

/// Cache file generik berkap maksimum (eviksi file terlama) — ganti path_provider.
/// Layout kunci: path relatif, mis. `$sourceId/$contentId/page_1.jpg`.
pub struct FileCacheDs {
    root: PathBuf,
    max_bytes: u64,
}

impl FileCacheDs {
    pub fn new(root: &Path, max_bytes: u64) -> Result<Self, AppError> {
        std::fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            max_bytes,
        })
    }

    fn resolve(&self, rel: &str) -> Result<PathBuf, AppError> {
        if rel.contains("..") || rel.starts_with('/') || rel.is_empty() {
            return Err(AppError::Validation(format!("cache key buruk: {rel}")));
        }
        Ok(self.root.join(rel))
    }

    pub fn put(&self, rel: &str, bytes: &[u8]) -> Result<(), AppError> {
        let path = self.resolve(rel)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        self.evict_if_needed()?;
        Ok(())
    }

    pub fn get(&self, rel: &str) -> Result<Option<Vec<u8>>, AppError> {
        let path = self.resolve(rel)?;
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read(&path)?))
    }

    pub fn total_bytes(&self) -> Result<u64, AppError> {
        Ok(walk_files(&self.root)?
            .iter()
            .map(|(_, size, _)| *size)
            .sum())
    }

    fn evict_if_needed(&self) -> Result<(), AppError> {
        let mut files = walk_files(&self.root)?;
        let mut total: u64 = files.iter().map(|(_, size, _)| *size).sum();
        files.sort_by_key(|(_, _, mtime)| *mtime);
        for (path, size, _) in files {
            if total <= self.max_bytes {
                break;
            }
            std::fs::remove_file(&path)?;
            total = total.saturating_sub(size);
        }
        Ok(())
    }
}

fn walk_files(root: &Path) -> Result<Vec<(PathBuf, u64, i64)>, AppError> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push((path, meta.len(), mtime));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SearchFilter;

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kuron-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_content() -> Content {
        Content {
            id: "m1".into(),
            title: "Sample".into(),
            cover_url: "https://cdn.example/c.jpg".into(),
            source_id: "nhentai".into(),
            upload_date: None,
            is_favorite: false,
            page_count: None,
            language: None,
        }
    }

    #[test]
    fn sqlite_crud_flow() {
        let ds = SqliteDs::open_in_memory().unwrap();
        // contents
        ds.save_content(&sample_content()).unwrap();
        let got = ds.get_content("m1").unwrap().unwrap();
        assert_eq!(got.title, "Sample");
        assert!(ds.get_content("hilang").unwrap().is_none());
        // chapters
        ds.save_chapters(&[Chapter {
            id: "c1".into(),
            content_id: "m1".into(),
            title: "Ch 1".into(),
            order: 1,
            is_external: false,
            external_url: None,
        }])
        .unwrap();
        // history
        ds.record_history("m1", 5).unwrap();
        let hist = ds.list_history(10).unwrap();
        assert_eq!(hist, vec![("m1".to_string(), 5)]);
        ds.clear_history().unwrap();
        assert!(ds.list_history(10).unwrap().is_empty());
        // favorites
        ds.set_favorite("m1", true).unwrap();
        assert_eq!(ds.list_favorites().unwrap(), vec!["m1".to_string()]);
        ds.set_favorite("m1", false).unwrap();
        assert!(ds.list_favorites().unwrap().is_empty());
        // downloads + translation cache
        ds.save_download("c1", "m1", r#"{"state":"done"}"#).unwrap();
        assert_eq!(
            ds.get_download("c1").unwrap().unwrap(),
            r#"{"state":"done"}"#
        );
        ds.put_translation("k1", b"data").unwrap();
        assert_eq!(ds.get_translation("k1").unwrap().unwrap(), b"data");
        // SearchFilter tetap serde-bersih (dipakai repo nanti).
        let _ = SearchFilter::default();
    }

    #[test]
    fn sqlite_file_persists() {
        let dir = tmpdir("sqlite");
        let db = dir.join("kuron.db");
        {
            let ds = SqliteDs::open(&db).unwrap();
            ds.save_content(&sample_content()).unwrap();
        }
        let ds2 = SqliteDs::open(&db).unwrap();
        assert!(ds2.get_content("m1").unwrap().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn kv_roundtrip_persists() {
        let dir = tmpdir("kv");
        let path = dir.join("kv.json");
        {
            let kv = KvStoreDs::open(&path).unwrap();
            kv.set("theme", "dark").unwrap();
            assert_eq!(kv.get("theme").as_deref(), Some("dark"));
        }
        let kv2 = KvStoreDs::open(&path).unwrap();
        assert_eq!(kv2.get("theme").as_deref(), Some("dark"));
        assert!(kv2.get("hilang").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_cache_put_get_evict() {
        let dir = tmpdir("cache");
        let cache = FileCacheDs::new(&dir, 10).unwrap();
        cache.put("a/1.jpg", b"12345").unwrap();
        assert_eq!(cache.get("a/1.jpg").unwrap().unwrap(), b"12345");
        assert!(cache.get("a/hilang.jpg").unwrap().is_none());
        assert!(cache.put("../evil", b"x").is_err());
        // Cap 10 byte: entry lama dieviksi setelah put kedua.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        cache.put("a/2.jpg", b"678901").unwrap();
        assert!(cache.total_bytes().unwrap() <= 10);
        assert!(cache.get("a/2.jpg").unwrap().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "menyentuh keychain OS; jalankan manual: cargo test secret_keychain -- --ignored"]
    fn secret_keychain_roundtrip() {
        let store = SecretStore::new("id.nhasix.kuron.test");
        store.delete_secret("akun-test").unwrap();
        assert!(store.get_secret("akun-test").unwrap().is_none());
        store.set_secret("akun-test", "s3cr3t").unwrap();
        assert_eq!(
            store.get_secret("akun-test").unwrap().as_deref(),
            Some("s3cr3t")
        );
        store.delete_secret("akun-test").unwrap();
        assert!(store.get_secret("akun-test").unwrap().is_none());
    }
}
