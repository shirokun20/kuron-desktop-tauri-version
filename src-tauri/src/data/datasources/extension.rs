//! Extension manager — port alur mobile `Settings > Sources > Extension Repository`.
//! Manifest (`manifest.json`) didaftar manual oleh pengguna (default: repo resmi
//! `kuron-extensions`); tiap sumber di-install sebagai JSON ke dir data aplikasi
//! (`{dir}/{id}-config.json`) setelah verifikasi checksum sha256 + parse valid.
//! TANPA config bawaan di bundle — app mulai kosong, user yang pasang.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    core::AppError,
    data::datasources::config::{SourceConfigs, SourceFile},
    network::HttpClientManager,
};

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_safe_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control())
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as usize;
        let b = chunk.get(1).copied().unwrap_or(0) as usize;
        let c = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(TABLE[a >> 2] as char);
        out.push(TABLE[((a & 3) << 4) | (b >> 4)] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((b & 15) << 2) | (c >> 6)] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[c & 63] as char
        } else {
            '='
        });
    }
    out
}

#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ZipSourceCandidate {
    pub id: String,
    pub version: String,
    pub display_name: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ZipPreview {
    pub token: String,
    pub sources: Vec<ZipSourceCandidate>,
}

/// Manifest resmi. Bisa diganti URL manifest lain (repo komunitas).
pub const DEFAULT_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/shirokun20/kuron-extensions/main/manifest.json";

#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ManifestMeta {
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub size_kb: u32,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
pub struct ManifestEntry {
    pub id: String,
    pub version: String,
    pub url: String,
    #[serde(default)]
    pub meta: Option<ManifestMeta>,
    #[serde(default)]
    pub checksum: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifest {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub minimum_app_version: String,
    #[serde(default)]
    pub installable_sources: Vec<ManifestEntry>,
}

impl ExtensionManifest {
    pub fn fetch(http: &HttpClientManager, manifest_url: &str) -> Result<Self, AppError> {
        let body = tauri::async_runtime::block_on(http.get(manifest_url, "manifest"))?;
        serde_json::from_str(&body)
            .map_err(|e| AppError::Network(format!("manifest {manifest_url}: {e}")))
    }

    /// Selesaikan URL relatif entry terhadap URL manifest.
    pub fn resolve_url(manifest_url: &str, entry_url: &str) -> String {
        if entry_url.starts_with("http://") || entry_url.starts_with("https://") {
            return entry_url.to_string();
        }
        match manifest_url.rfind('/') {
            Some(i) => format!(
                "{}/{}",
                &manifest_url[..i],
                entry_url.trim_start_matches('/')
            ),
            None => entry_url.to_string(),
        }
    }
}

pub struct ExtensionManager<'a> {
    http: &'a HttpClientManager,
    dir: std::path::PathBuf,
}

impl<'a> ExtensionManager<'a> {
    pub fn new(http: &'a HttpClientManager, dir: &std::path::Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            http,
            dir: dir.to_path_buf(),
        })
    }

    fn path_for(&self, id: &str) -> std::path::PathBuf {
        self.dir.join(format!("{id}-config.json"))
    }

    /// Install satu sumber: unduh JSON → cek sha256 → parse valid → simpan.
    pub fn install(
        &self,
        manifest_url: &str,
        entry: &ManifestEntry,
    ) -> Result<SourceFile, AppError> {
        if !is_safe_component(&entry.id) {
            return Err(AppError::Validation(format!(
                "id ekstensi buruk: {}",
                entry.id
            )));
        }
        let url = ExtensionManifest::resolve_url(manifest_url, &entry.url);
        let body = tauri::async_runtime::block_on(self.http.get(&url, &entry.id))?;
        if body.is_empty() {
            return Err(AppError::Network(format!("config kosong: {url}")));
        }
        if !entry.checksum.is_empty() {
            let digest = sha256_hex(body.as_bytes());
            if digest != entry.checksum.to_lowercase() {
                return Err(AppError::Validation(format!(
                    "checksum {} tak cocok (manifest vs unduhan)",
                    entry.id
                )));
            }
        }
        let cfg: SourceFile = serde_json::from_str(&body)
            .map_err(|e| AppError::Validation(format!("config {} tidak valid: {e}", entry.id)))?;
        std::fs::write(self.path_for(&entry.id), &body)?;
        // Simpan meta ikon (absolute) untuk tile UI; tak ada → None.
        let icon_url = entry
            .meta
            .as_ref()
            .and_then(|m| m.icon_url.clone())
            .map(|u| ExtensionManifest::resolve_url(manifest_url, &u));
        let meta_path = self.dir.join(format!("{}-meta.json", entry.id));
        let meta_json = serde_json::json!({ "icon_url": icon_url });
        std::fs::write(meta_path, meta_json.to_string())?;
        Ok(cfg)
    }

    pub fn uninstall(&self, id: &str) -> Result<(), AppError> {
        let path = self.path_for(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let meta = self.dir.join(format!("{id}-meta.json"));
        if meta.exists() {
            std::fs::remove_file(&meta)?;
        }
        Ok(())
    }

    /// Install dari URL zip (struktur `kuron-extensions`: berisi `*-config.json`).
    pub fn install_zip_url(&self, url: &str) -> Result<Vec<String>, AppError> {
        let bytes = tauri::async_runtime::block_on(self.http.get_bytes(url, "extensions", None))?;
        self.install_zip_bytes(&bytes)
    }

    pub fn preview_zip_bytes(&self, bytes: &[u8]) -> Result<ZipPreview, AppError> {
        let token = sha256_hex(bytes);
        let candidates = self.inspect_zip(bytes)?;
        std::fs::write(self.dir.join(format!(".zip-preview-{token}")), bytes)?;
        Ok(ZipPreview {
            token,
            sources: candidates,
        })
    }

    pub fn install_staged_zip(
        &self,
        token: &str,
        selected: &[String],
    ) -> Result<Vec<String>, AppError> {
        if !is_safe_component(token) || token.len() != 64 {
            return Err(AppError::Validation("token zip buruk".to_string()));
        }
        if selected.is_empty() {
            return Err(AppError::Validation(
                "pilih minimal satu sumber".to_string(),
            ));
        }
        let path = self.dir.join(format!(".zip-preview-{token}"));
        let bytes = std::fs::read(&path)?;
        let result = self.install_zip_bytes_selected(&bytes, selected)?;
        let _ = std::fs::remove_file(path);
        Ok(result)
    }

    /// Install dari bytes zip: tiap `*-config.json` divalidasi parse lalu disimpan
    /// sebagai `{source}-config.json`. Nama path diabaikan (anti zip-slip).
    /// Entri bukan JSON config dilewati.
    pub fn install_zip_bytes(&self, bytes: &[u8]) -> Result<Vec<String>, AppError> {
        let ids = self
            .inspect_zip(bytes)?
            .into_iter()
            .map(|s| s.id)
            .collect::<Vec<_>>();
        self.install_zip_bytes_selected(bytes, &ids)
    }

    fn install_zip_bytes_selected(
        &self,
        bytes: &[u8],
        selected: &[String],
    ) -> Result<Vec<String>, AppError> {
        let configs = self.inspect_zip_with_icons(bytes)?;
        let selected: std::collections::HashSet<&str> =
            selected.iter().map(String::as_str).collect();
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AppError::Validation(format!("zip rusak: {e}")))?;
        let mut staged: Vec<(String, String)> = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::Validation(format!("zip entry: {e}")))?;
            let name = file.name().to_string();
            if !name.ends_with("-config.json") {
                continue;
            }
            let mut raw = String::new();
            use std::io::Read;
            file.read_to_string(&mut raw)
                .map_err(|e| AppError::Validation(format!("zip baca {name}: {e}")))?;
            let cfg: SourceFile = serde_json::from_str(&raw)
                .map_err(|e| AppError::Validation(format!("config {name} tidak valid: {e}")))?;
            if !is_safe_component(&cfg.source) {
                return Err(AppError::Validation(format!("source buruk di {name}")));
            }
            if !selected.contains(cfg.source.as_str()) {
                continue;
            }
            let source = cfg.source.clone();
            let icon_url = configs
                .iter()
                .find(|candidate| candidate.0.id == source)
                .and_then(|candidate| candidate.1.clone());
            staged.push((source.clone(), raw));
            if let Some(icon_url) = icon_url {
                let meta_path = self.dir.join(format!("{source}-meta.json"));
                std::fs::write(
                    meta_path,
                    serde_json::json!({ "icon_url": icon_url }).to_string(),
                )?;
            }
        }
        let mut installed = Vec::new();
        for (source, raw) in staged {
            std::fs::write(self.path_for(&source), &raw)?;
            installed.push(source);
        }
        installed.sort();
        Ok(installed)
    }

    fn inspect_zip(&self, bytes: &[u8]) -> Result<Vec<ZipSourceCandidate>, AppError> {
        Ok(self
            .inspect_zip_with_icons(bytes)?
            .into_iter()
            .map(|(candidate, _)| candidate)
            .collect())
    }

    fn inspect_zip_with_icons(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<(ZipSourceCandidate, Option<String>)>, AppError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AppError::Validation(format!("zip rusak: {e}")))?;
        let mut out = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::Validation(format!("zip entry: {e}")))?;
            let name = file.name().to_string();
            if !name.ends_with("-config.json") {
                continue;
            }
            let mut raw = String::new();
            use std::io::Read;
            file.read_to_string(&mut raw)
                .map_err(|e| AppError::Validation(format!("zip baca {name}: {e}")))?;
            drop(file);
            let cfg: SourceFile = serde_json::from_str(&raw)
                .map_err(|e| AppError::Validation(format!("config {name} tidak valid: {e}")))?;
            if !is_safe_component(&cfg.source) {
                return Err(AppError::Validation(format!("source buruk di {name}")));
            }
            let icon = cfg.ui_icon_path().and_then(|path| {
                if path.starts_with("http://") || path.starts_with("https://") {
                    return Some(path);
                }
                let icon_name = path.trim_start_matches("./");
                let mut icon_file = archive.by_name(icon_name).ok()?;
                let mut data = Vec::new();
                icon_file.read_to_end(&mut data).ok()?;
                let mime = if icon_name.ends_with(".svg") {
                    "image/svg+xml"
                } else if icon_name.ends_with(".jpg") || icon_name.ends_with(".jpeg") {
                    "image/jpeg"
                } else {
                    "image/png"
                };
                Some(format!("data:{mime};base64,{}", base64_encode(&data)))
            });
            let source = cfg.source.clone();
            let version = cfg.version.clone();
            out.push((
                ZipSourceCandidate {
                    id: source,
                    version,
                    display_name: cfg.ui_display_name(),
                    icon_url: icon.clone(),
                },
                icon,
            ));
        }
        Ok(out)
    }

    pub fn installed_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("json") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        if let Some(id) = stem.strip_suffix("-config") {
                            ids.push(id.to_string());
                        }
                    }
                }
            }
        }
        ids.sort();
        ids
    }

    pub fn load_installed(&self) -> Result<SourceConfigs, AppError> {
        SourceConfigs::load_dir(&self.dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    const MANIFEST_JSON: &str = r#"{"schemaVersion": 2, "installableSources": [
        {"id": "tes", "version": "1.0.0", "url": "config/tes-config.json",
         "meta": {"displayName": "Tes"}, "checksum": "__CHECKSUM__"}
    ]}"#;

    const CONFIG_JSON: &str = r#"{"source": "tes", "version": "1.0.0",
        "baseUrl": "https://tes.example",
        "api": {"enabled": true, "apiBase": "https://tes.example",
                "endpoints": {"search": "/s?q={query}"}}}"#;

    fn tmpdir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kuron-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Server fixture: `/manifest.json` + `/config/tes-config.json`.
    fn spawn_repo() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            for stream in listener.incoming().take(8) {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 2048];
                let len = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..len]).to_string();
                let path = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/");
                let checksum = sha256_hex(CONFIG_JSON.as_bytes());
                let body = if path.ends_with("manifest.json") {
                    MANIFEST_JSON.replace("__CHECKSUM__", &checksum)
                } else {
                    CONFIG_JSON.to_string()
                };
                let res = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(res.as_bytes()).unwrap();
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn install_verifies_checksum_and_caches() {
        let base = spawn_repo();
        let http = HttpClientManager::without_proxy().unwrap();
        let manifest_url = format!("{base}/manifest.json");
        let manifest = ExtensionManifest::fetch(&http, &manifest_url).unwrap();
        assert_eq!(manifest.installable_sources.len(), 1);
        let entry = &manifest.installable_sources[0];
        assert_eq!(
            ExtensionManifest::resolve_url(&manifest_url, &entry.url),
            format!("{base}/config/tes-config.json")
        );

        let dir = tmpdir("ext");
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        assert!(mgr.installed_ids().is_empty());
        let cfg = mgr.install(&manifest_url, entry).unwrap();
        assert_eq!(cfg.source, "tes");
        assert_eq!(mgr.installed_ids(), vec!["tes".to_string()]);
        let loaded = mgr.load_installed().unwrap();
        assert!(loaded.get("tes").is_some());

        // Checksum salah ditolak, file lama utuh.
        let mut bad = entry.clone();
        bad.checksum = "00".repeat(32);
        assert!(mgr.install(&manifest_url, &bad).is_err());
        assert!(mgr.load_installed().unwrap().get("tes").is_some());

        // Uninstall bersih.
        mgr.uninstall("tes").unwrap();
        assert!(mgr.installed_ids().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_zip_bytes_extracts_configs() {
        let dir = tmpdir("ext-zip");
        let http = HttpClientManager::without_proxy().unwrap();
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        // Zip kotor: 1 config valid + 1 non-config (dilewati) + 1 entry
        // traversal berisi JSON invalid → seluruh zip ditolak, nihil tertulis.
        let mut dirty = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut dirty);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            use std::io::Write;
            w.start_file("nested/hitomi-config.json", opts).unwrap();
            w.write_all(
                br#"{"source": "hitomi", "version": "1.0.5", "baseUrl": "https://hitomi.la"}"#,
            )
            .unwrap();
            w.start_file("README.md", opts).unwrap();
            w.write_all(b"abaikan").unwrap();
            w.start_file("../../evil-config.json", opts).unwrap();
            w.write_all(b"{}").unwrap();
            w.finish().unwrap();
        }
        assert!(mgr.install_zip_bytes(dirty.get_ref()).is_err());
        assert!(mgr.installed_ids().is_empty());
        // Zip bersih → terpasang.
        let mut clean = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut clean);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            use std::io::Write;
            w.start_file("hitomi-config.json", opts).unwrap();
            w.write_all(
                br#"{"source": "hitomi", "version": "1.0.5", "baseUrl": "https://hitomi.la"}"#,
            )
            .unwrap();
            w.finish().unwrap();
        }
        let ids = mgr.install_zip_bytes(clean.get_ref()).unwrap();
        assert_eq!(ids, vec!["hitomi".to_string()]);
        assert!(mgr.load_installed().unwrap().get("hitomi").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_zip_accepts_domain_style_source_ids() {
        let dir = tmpdir("ext-zip-domain");
        let http = HttpClientManager::without_proxy().unwrap();
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        let mut zip = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut zip);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            w.start_file("config/manga18.club-config.json", opts)
                .unwrap();
            w.write_all(
                br#"{"source":"manga18.club","version":"1.0.0","baseUrl":"https://manga18.club"}"#,
            )
            .unwrap();
            w.finish().unwrap();
        }
        let ids = mgr.install_zip_bytes(zip.get_ref()).unwrap();
        assert_eq!(ids, vec!["manga18.club".to_string()]);
        assert!(mgr.load_installed().unwrap().get("manga18.club").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_preview_selects_sources_and_resolves_local_icons() {
        let dir = tmpdir("ext-zip-preview");
        let http = HttpClientManager::without_proxy().unwrap();
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        let mut zip = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut zip);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            use std::io::Write;
            w.start_file("images/a.png", opts).unwrap();
            w.write_all(&[137, 80, 78, 71]).unwrap();
            w.start_file("a-config.json", opts).unwrap();
            w.write_all(
                br#"{"source":"a","version":"1","ui":{"displayName":"A","iconPath":"./images/a.png"}}"#,
            )
            .unwrap();
            w.start_file("b-config.json", opts).unwrap();
            w.write_all(br#"{"source":"b","version":"1"}"#).unwrap();
            w.finish().unwrap();
        }
        let preview = mgr.preview_zip_bytes(zip.get_ref()).unwrap();
        assert_eq!(preview.sources.len(), 2);
        assert!(preview.sources[0]
            .icon_url
            .as_deref()
            .is_some_and(|icon| icon.starts_with("data:image/png;base64,")));
        let installed = mgr
            .install_staged_zip(&preview.token, &["a".to_string()])
            .unwrap();
        assert_eq!(installed, vec!["a".to_string()]);
        assert!(mgr.load_installed().unwrap().get("a").is_some());
        assert!(mgr.load_installed().unwrap().get("b").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "hits live kuron-extensions; jalankan manual: cargo test live_manifest_install -- --ignored"]
    fn live_manifest_install_roundtrip() {
        let http = HttpClientManager::new().unwrap();
        let manifest = ExtensionManifest::fetch(&http, DEFAULT_MANIFEST_URL).unwrap();
        assert!(!manifest.installable_sources.is_empty());
        let entry = manifest
            .installable_sources
            .iter()
            .filter(|e| !e.id.is_empty())
            .min_by_key(|e| e.meta.as_ref().map(|m| m.size_kb).unwrap_or(9999))
            .unwrap()
            .clone();
        let dir = tmpdir("ext-live");
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        let cfg = mgr.install(DEFAULT_MANIFEST_URL, &entry).unwrap();
        assert!(!cfg.base_url.is_empty());
        assert!(mgr.load_installed().unwrap().get(&entry.id).is_some());
        mgr.uninstall(&entry.id).unwrap();
        assert!(mgr.installed_ids().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_bad_ids() {
        let dir = tmpdir("ext-bad");
        let http = HttpClientManager::without_proxy().unwrap();
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        let evil = ManifestEntry {
            id: "../evil".into(),
            version: "1".into(),
            url: "x".into(),
            meta: None,
            checksum: String::new(),
        };
        assert!(mgr
            .install("http://localhost/manifest.json", &evil)
            .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
