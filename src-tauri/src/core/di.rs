//! AppState: composition root payload, di-`manage` di lib.rs.
//! Ganti `get_it` `service_locator.dart`. Repo default mock; `repo_for(source)`
//! membangun repo real dari config ter-install (4.7) — feed ikut sumber terpasang.

use std::{path::PathBuf, sync::Arc};

use crate::{
    data::{
        datasources::{
            config::SourceConfigs,
            remote::{
                ehentai_config, hitomi_config, FieldMap, GenericRestAdapter,
                GenericScraperAdapter, NhentaiApiAdapter, SourceConfig,
            },
        },
        repositories::{ContentRepositoryImpl, MockContentRepository},
    },
    domain::repositories::ContentRepository,
    network::HttpClientManager,
};

pub struct AppState {
    pub app_name: String,
    pub version: String,
    /// Default legacy (mock) — dipakai feed "Semua".
    pub content_repo: Arc<dyn ContentRepository>,
    pub http: HttpClientManager,
    /// Dir config ekstensi ter-install (`{data}/extensions`).
    pub ext_dir: PathBuf,
}

fn default_ext_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("id.nhasix.kuron")
        .join("extensions")
}

impl AppState {
    pub fn new(content_repo: Arc<dyn ContentRepository>) -> Self {
        let ext_dir = default_ext_dir();
        let _ = std::fs::create_dir_all(&ext_dir);
        Self {
            app_name: "Kuron Desktop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            content_repo,
            http: HttpClientManager::new().expect("tls backend init"),
            ext_dir,
        }
    }

    fn scraper_config_for(source_id: &str) -> Option<SourceConfig> {
        match source_id {
            "hitomi" => Some(hitomi_config()),
            "ehentai" => Some(ehentai_config()),
            "mangadex" => Some(SourceConfig {
                source_id: "mangadex".to_string(),
                base_url: "https://api.mangadex.org".to_string(),
                list_path: String::new(),
                home_path: String::new(),
                home_page_path: String::new(),
                detail_path: String::new(),
                item_selector: String::new(),
                id: FieldMap::default(),
                title: FieldMap::default(),
                link: FieldMap::default(),
                cover: FieldMap::default(),
                detail_title: FieldMap::default(),
                detail_cover: FieldMap::default(),
                chapter_selector: String::new(),
                chapter_link: FieldMap::default(),
                chapter_title: FieldMap::default(),
                page_selector: String::new(),
                page_attr: None,
                page_count: FieldMap::default(),
                language: FieldMap::default(),
                default_language: None,
            }),
            _ => None,
        }
    }

    /// Repo untuk satu sumber ter-install (atau bawaan). Error bila tak dikenal.
    pub fn repo_for(&self, source_id: &str) -> Result<Arc<dyn ContentRepository>, crate::core::AppError> {
        use crate::core::AppError;
        if source_id == "semua" {
            return Ok(self.content_repo.clone());
        }
        let overlay =
            SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &self.ext_dir)?;
        let source_cfg = overlay.get(source_id).ok_or_else(|| {
            AppError::Validation(format!(
                "sumber '{source_id}' belum ter-install — pasang via Ekstensi"
            ))
        })?;
        let http = self.http.clone();
        let scraper = GenericScraperAdapter::new(http.clone());
        let rest = GenericRestAdapter::new(http.clone());
        if source_id == "nhentai" {
            let api = NhentaiApiAdapter::from_config(http, source_cfg);
            let scfg = Self::scraper_config_for("nhentai").unwrap_or(SourceConfig {
                source_id: "nhentai".to_string(),
                base_url: "https://nhentai.net".to_string(),
                list_path: String::new(),
                home_path: String::new(),
                home_page_path: String::new(),
                detail_path: String::new(),
                item_selector: String::new(),
                id: FieldMap::default(),
                title: FieldMap::default(),
                link: FieldMap::default(),
                cover: FieldMap::default(),
                detail_title: FieldMap::default(),
                detail_cover: FieldMap::default(),
                chapter_selector: String::new(),
                chapter_link: FieldMap::default(),
                chapter_title: FieldMap::default(),
                page_selector: String::new(),
                page_attr: None,
                page_count: FieldMap::default(),
                language: FieldMap::default(),
                default_language: None,
            });
            return Ok(Arc::new(
                ContentRepositoryImpl::new(scraper, rest, scfg).with_nhentai(api),
            ));
        }
        match Self::scraper_config_for(source_id) {
            Some(scfg) => Ok(Arc::new(ContentRepositoryImpl::new(scraper, rest, scfg))),
            None => {
                // JSON config ter-install (areakomik dkk) → engine generik.
                // Hardcode hanya fallback bila JSON tak punya pola list.
                if let Some(file) = overlay.get(source_id) {
                    if let Some(scraper_sec) = file.scraper.as_ref() {
                        if let Some(scfg) = crate::data::datasources::remote::source_config_from_json(
                            source_id,
                            &file.base_url,
                            scraper_sec,
                            &file.default_language,
                        ) {
                            return Ok(Arc::new(ContentRepositoryImpl::new(scraper, rest, scfg)));
                        }
                    }
                }
                Err(AppError::Validation(format!(
                    "sumber '{source_id}' ter-install tapi pola scraper-nya tak didukung engine"
                )))
            }
        }
    }

    /// Daftar sumber efektif (bundled + installed) untuk UI.
    pub fn source_entries(&self) -> Result<Vec<InstalledSource>, crate::core::AppError> {
        let overlay =
            SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &self.ext_dir)?;
        let mut out: Vec<InstalledSource> = overlay
            .iter()
            .map(|(id, cfg)| {
                let meta_path = self.ext_dir.join(format!("{id}-meta.json"));
                let icon_url = std::fs::read_to_string(&meta_path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .and_then(|v| v.get("icon_url")?.as_str().map(String::from));
                InstalledSource {
                    id: id.clone(),
                    version: cfg.version.clone(),
                    base_url: cfg.base_url.clone(),
                    installed: self.ext_dir.join(format!("{id}-config.json")).exists(),
                    icon_url,
                }
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }
}

/// Ringkasan satu sumber untuk daftar dinamis UI (ts-rs → TS).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct InstalledSource {
    pub id: String,
    pub version: String,
    pub base_url: String,
    pub installed: bool,
    pub icon_url: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Arc::new(MockContentRepository))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::HomeFeedRepository;

    #[test]
    fn default_state_serves_mock_feed() {
        let feed =
            tauri::async_runtime::block_on(AppState::default().content_repo.home_feed(1))
                .unwrap();
        assert_eq!(feed.len(), 8);
    }

    #[test]
    fn unknown_source_errors_with_hint() {
        let state = AppState::default();
        match state.repo_for("tidak-ada") {
            Err(e) => assert!(e.to_string().contains("belum ter-install"), "{e}"),
            Ok(_) => panic!("seharusnya error"),
        }
    }

    #[test]
    #[ignore = "hits live areakomik + kuron-extensions; manual: cargo test live_areakomik -- --ignored"]
    fn live_areakomik_feed_via_installed_json() {
        use crate::data::datasources::{
            config::SourceConfigs,
            extension::{ExtensionManager, ExtensionManifest, DEFAULT_MANIFEST_URL},
            remote::{source_config_from_json, GenericRestAdapter, GenericScraperAdapter},
        };
        use crate::domain::repositories::HomeFeedRepository;
        let http = HttpClientManager::new().unwrap();
        let manifest = ExtensionManifest::fetch(&http, DEFAULT_MANIFEST_URL).unwrap();
        let entry = manifest
            .installable_sources
            .iter()
            .find(|e| e.id == "areakomik")
            .expect("areakomik ada di manifest")
            .clone();
        let dir = std::env::temp_dir().join(format!("kuron-test-ak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mgr = ExtensionManager::new(&http, &dir).unwrap();
        let cfg_file = mgr.install(DEFAULT_MANIFEST_URL, &entry).unwrap();
        let overlay = SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &dir).unwrap();
        assert!(overlay.get("areakomik").is_some());
        let scraper_sec = cfg_file.scraper.as_ref().expect("areakomik punya scraper");
        let scfg =
            source_config_from_json("areakomik", &cfg_file.base_url, scraper_sec, &cfg_file.default_language)
                .expect("pola areakomik terbaca");
        let repo = ContentRepositoryImpl::new(
            GenericScraperAdapter::new(http.clone()),
            GenericRestAdapter::new(http),
            scfg,
        );
        let feed = tauri::async_runtime::block_on(repo.home_feed(1)).unwrap();
        assert!(!feed.is_empty(), "feed areakomik live terisi");
        assert!(!feed[0].title.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bundled_nhentai_resolves() {
        let state = AppState::default();
        assert!(state.repo_for("nhentai").is_ok());
        let entries = state.source_entries().unwrap();
        assert!(entries.iter().any(|e| e.id == "nhentai" && !e.version.is_empty()));
    }
}
