//! AppState: composition root payload, di-`manage` di lib.rs.
//! Ganti `get_it` `service_locator.dart`. Repo default mock; `repo_for(source)`
//! membangun repo real dari config ter-install (4.7) — feed ikut sumber terpasang.

use std::{path::PathBuf, sync::Arc};

use crate::{
    core::constants::APP_ID,
    data::{
        datasources::{
            config::SourceConfigs,
            local::{KvStoreDs, SecretStore},
            remote::{
                ehentai_config, hitomi_config, FieldMap, GenericRestAdapter, GenericScraperAdapter,
                NhentaiApiAdapter, PaginationCursors, SourceConfig,
            },
        },
        repositories::{
            AiProviderRepositoryImpl, ContentRepositoryImpl, LibraryRepositoryImpl,
            MockContentRepository,
        },
    },
    domain::repositories::{AiProviderRepository, ContentRepository, LibraryRepository},
    network::HttpClientManager,
};

pub struct AppState {
    pub app_name: String,
    pub version: String,
    /// Default legacy (mock) — dipakai feed "Semua".
    pub content_repo: Arc<dyn ContentRepository>,
    /// Library lokal SQLite (riwayat + favorit, 8.3).
    pub library: Arc<dyn LibraryRepository>,
    /// Provider AI BYOK: metadata di KV, kunci di keychain OS (9.2).
    pub ai_providers: Arc<dyn AiProviderRepository>,
    pub http: HttpClientManager,
    /// Dir config ekstensi ter-install (`{data}/extensions`).
    pub ext_dir: PathBuf,
    /// Cursor pagination token per sumber (E-Hentai `?next=`, ala mobile).
    pub cursors: PaginationCursors,
}

fn default_ext_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("id.nhasix.kuron")
        .join("extensions")
}

fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("id.nhasix.kuron")
        .join("kuron.db")
}

fn default_kv_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("id.nhasix.kuron")
        .join("kv.json")
}

impl AppState {
    pub fn new(content_repo: Arc<dyn ContentRepository>) -> Self {
        let ext_dir = default_ext_dir();
        let _ = std::fs::create_dir_all(&ext_dir);
        // Library: file `kuron.db`; bila gagal dibuka (lock/IO), fallback
        // memori + warn agar app tetap boot (data sesi tak persist).
        let library: Arc<dyn LibraryRepository> =
            match LibraryRepositoryImpl::open(&default_db_path()) {
                Ok(repo) => Arc::new(repo),
                Err(e) => {
                    tracing::warn!("library fallback memori: {e}");
                    Arc::new(LibraryRepositoryImpl::open_in_memory().expect("sqlite memori init"))
                }
            };
        // AI provider BYOK: metadata di KV file; kunci hanya di keychain OS.
        let ai_providers: Arc<dyn AiProviderRepository> = Arc::new(AiProviderRepositoryImpl::new(
            KvStoreDs::open(&default_kv_path()).expect("kv init"),
            SecretStore::new(APP_ID),
        ));
        Self {
            app_name: "Kuron Desktop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            content_repo,
            library,
            ai_providers,
            http: HttpClientManager::new().expect("tls backend init"),
            ext_dir,
            cursors: PaginationCursors::default(),
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
                pagination_next: String::new(),
                detail_path: String::new(),
                item_selector: String::new(),
                id: FieldMap::default(),
                title: FieldMap::default(),
                link: FieldMap::default(),
                cover: FieldMap::default(),
                detail_title: FieldMap::default(),
                detail_cover: FieldMap::default(),
                detail_page_count: FieldMap::default(),
                detail_description: FieldMap::default(),
                detail_genres: FieldMap::default(),
                detail_language: FieldMap::default(),
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
    pub fn repo_for(
        &self,
        source_id: &str,
    ) -> Result<Arc<dyn ContentRepository>, crate::core::AppError> {
        use crate::core::AppError;
        if source_id == "semua" {
            return Ok(self.content_repo.clone());
        }
        let overlay = SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &self.ext_dir)?;
        let source_cfg = overlay.get(source_id).ok_or_else(|| {
            AppError::Validation(format!(
                "sumber '{source_id}' belum ter-install — pasang via Ekstensi"
            ))
        })?;
        let http = self.http.clone();
        let scraper = GenericScraperAdapter::new(http.clone());
        let rest = GenericRestAdapter::new(http.clone());
        // REST adapter ikut JSON sumber (endpoint + queryRules config-driven);
        // bila tak ada `api`, fallback template bawaan per sumber.
        let rest = match source_id {
            "mangadex" | "hitomi" | "ehentai" | "nhentai" => {
                rest.with_source_file(source_cfg.clone())
            }
            _ => rest,
        };
        if source_id == "nhentai" {
            let api = NhentaiApiAdapter::from_config(http, source_cfg);
            let scfg = Self::scraper_config_for("nhentai").unwrap_or(SourceConfig {
                source_id: "nhentai".to_string(),
                base_url: "https://nhentai.net".to_string(),
                list_path: String::new(),
                home_path: String::new(),
                home_page_path: String::new(),
                pagination_next: String::new(),
                detail_path: String::new(),
                item_selector: String::new(),
                id: FieldMap::default(),
                title: FieldMap::default(),
                link: FieldMap::default(),
                cover: FieldMap::default(),
                detail_title: FieldMap::default(),
                detail_cover: FieldMap::default(),
                detail_page_count: FieldMap::default(),
                detail_description: FieldMap::default(),
                detail_genres: FieldMap::default(),
                detail_language: FieldMap::default(),
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
                ContentRepositoryImpl::new(scraper, rest, scfg)
                    .with_nhentai(api)
                    .with_pagination(self.cursors.clone()),
            ));
        }
        match source_id {
            // JSON bundled/installed menang; hardcode hanya bila JSON tak
            // punya pola list (dulu terbalik: hardcode selalu menang).
            _ if source_id == "mangadex" => {
                let scfg = Self::scraper_config_for("mangadex").unwrap_or(SourceConfig {
                    source_id: "mangadex".to_string(),
                    base_url: "https://api.mangadex.org".to_string(),
                    list_path: String::new(),
                    home_path: String::new(),
                    home_page_path: String::new(),
                    pagination_next: String::new(),
                    detail_path: String::new(),
                    item_selector: String::new(),
                    id: FieldMap::default(),
                    title: FieldMap::default(),
                    link: FieldMap::default(),
                    cover: FieldMap::default(),
                    detail_title: FieldMap::default(),
                    detail_cover: FieldMap::default(),
                    detail_page_count: FieldMap::default(),
                    detail_description: FieldMap::default(),
                    detail_genres: FieldMap::default(),
                    detail_language: FieldMap::default(),
                    chapter_selector: String::new(),
                    chapter_link: FieldMap::default(),
                    chapter_title: FieldMap::default(),
                    page_selector: String::new(),
                    page_attr: None,
                    page_count: FieldMap::default(),
                    language: FieldMap::default(),
                    default_language: None,
                });
                Ok(Arc::new(
                    ContentRepositoryImpl::new(scraper, rest, scfg)
                        .with_pagination(self.cursors.clone()),
                ))
            }
            _ => {
                // JSON config (ehentai/hitomi/areakomik dkk) → engine generik.
                if let Some(file) = overlay.get(source_id) {
                    if let Some(scraper_sec) = file.scraper.as_ref() {
                        if let Some(scfg) =
                            crate::data::datasources::remote::source_config_from_json(
                                source_id,
                                &file.base_url,
                                scraper_sec,
                                &file.default_language,
                            )
                        {
                            return Ok(Arc::new(
                                ContentRepositoryImpl::new(scraper, rest, scfg)
                                    .with_pagination(self.cursors.clone()),
                            ));
                        }
                    }
                }
                // Hardcode hanya fallback bila JSON tak punya pola list.
                match Self::scraper_config_for(source_id) {
                    Some(scfg) => Ok(Arc::new(
                        ContentRepositoryImpl::new(scraper, rest, scfg)
                            .with_pagination(self.cursors.clone()),
                    )),
                    None => Err(AppError::Validation(format!(
                        "sumber '{source_id}' ter-install tapi pola scraper-nya tak didukung engine"
                    ))),
                }
            }
        }
    }

    /// Daftar sumber efektif (bundled + installed) untuk UI.
    /// Ikon per sumber: meta.json hasil install dulu (URL absolut dari
    /// manifest), lalu `ui.iconPath` dari config itu sendiri (bundled/lokal),
    /// terakhir None (UI pakai fallback inisial).
    pub fn source_entries(&self) -> Result<Vec<InstalledSource>, crate::core::AppError> {
        let overlay = SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &self.ext_dir)?;
        let mut out: Vec<InstalledSource> = overlay
            .iter()
            .map(|(id, cfg)| {
                let meta_path = self.ext_dir.join(format!("{id}-meta.json"));
                let icon_url = std::fs::read_to_string(&meta_path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .and_then(|v| v.get("icon_url")?.as_str().map(String::from))
                    .or_else(|| cfg.ui_icon_path());
                let display_name = cfg.ui_display_name();
                InstalledSource {
                    id: id.clone(),
                    version: cfg.version.clone(),
                    base_url: cfg.base_url.clone(),
                    installed: self.ext_dir.join(format!("{id}-config.json")).exists(),
                    icon_url,
                    display_name,
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
    /// `ui.displayName` dari config sumber (nama tampil ala mobile).
    pub display_name: Option<String>,
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
            tauri::async_runtime::block_on(AppState::default().content_repo.home_feed(1)).unwrap();
        assert_eq!(feed.len(), 8);
    }

    /// Live: rating erotica via repo + config installed (butuh internet).
    /// Config mangadex di-install dari manifest resmi ke temp dir
    /// (hermetik, config-driven — tanpa bundle/data-dir pengguna).
    #[cfg(feature = "live-tests")]
    #[test]
    fn live_repro_md_rating() {
        use crate::data::datasources::extension::{
            ExtensionManager, ExtensionManifest, DEFAULT_MANIFEST_URL,
        };
        use crate::domain::{repositories::ContentRepository, SearchFilter};
        let http = HttpClientManager::new().unwrap();
        let manifest = ExtensionManifest::fetch(&http, DEFAULT_MANIFEST_URL).unwrap();
        let entry = manifest
            .installable_sources
            .iter()
            .find(|e| e.id == "mangadex")
            .expect("mangadex ada di manifest")
            .clone();
        let dir = std::env::temp_dir().join(format!("kuron-test-md-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ExtensionManager::new(&http, &dir)
            .unwrap()
            .install(DEFAULT_MANIFEST_URL, &entry)
            .unwrap();
        let state = state_with_ext_dir(&dir);
        let repo = state.repo_for("mangadex").unwrap();
        for q in [
            "raw:contentRating[]=erotica",
            "raw:contentRating%5B%5D=erotica",
        ] {
            let filter = SearchFilter {
                query: q.to_string(),
                source_id: Some("mangadex".to_string()),
                page: 1,
            };
            let items =
                tauri::async_runtime::block_on(async { repo.search(filter).await }).unwrap();
            assert!(!items.is_empty(), "rating {q} kosong");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_source_errors_with_hint() {
        let state = AppState::default();
        match state.repo_for("tidak-ada") {
            Err(e) => assert!(e.to_string().contains("belum ter-install"), "{e}"),
            Ok(_) => panic!("seharusnya error"),
        }
    }

    /// AppState dengan ext_dir fixture (pola sama dengan `config.rs::write_fixture`).
    fn state_with_ext_dir(dir: &std::path::Path) -> AppState {
        let _ = std::fs::create_dir_all(dir);
        AppState {
            app_name: "test".into(),
            version: "0".into(),
            content_repo: Arc::new(MockContentRepository),
            library: Arc::new(LibraryRepositoryImpl::open_in_memory().unwrap()),
            ai_providers: Arc::new(AiProviderRepositoryImpl::new(
                KvStoreDs::open(&dir.join("kv.json")).unwrap(),
                SecretStore::new_memory(),
            )),
            http: HttpClientManager::new().expect("tls backend init"),
            ext_dir: dir.to_path_buf(),
            cursors: PaginationCursors::default(),
        }
    }

    /// Config fixture: bundled `ui.iconPath` lokal harus terbaca sebagai ikon.
    #[test]
    fn source_entries_reads_icon_from_bundled_config() {
        let dir = std::env::temp_dir().join(format!("kuron-icon-bundled-{}", std::process::id()));
        let state = state_with_ext_dir(&dir);
        let entries = state.source_entries().unwrap();
        let nh = entries
            .iter()
            .find(|s| s.id == "nhentai")
            .expect("bundled nhentai harus ada");
        let icon = nh.icon_url.as_deref().expect("ui.iconPath bundled terbaca");
        assert!(icon.contains("nhentai.png"), "ikon nhentai: {icon}");
        assert_eq!(nh.display_name.as_deref(), Some("NHentai"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Sumber ter-install: meta.json (dari manifest) MENANG atas ui.iconPath
    /// config, dan display_name tetap dari config.
    #[test]
    fn installed_meta_icon_overrides_bundled_config_icon() {
        let dir = std::env::temp_dir().join(format!("kuron-icon-installed-{}", std::process::id()));
        let state = state_with_ext_dir(&dir);
        std::fs::write(
            dir.join("nhentai-config.json"),
            r#"{"source":"nhentai","version":"1.2.3","baseUrl":"https://i.nhentai.net","ui":{"displayName":"NHentai","iconPath":"config-icon.png"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("nhentai-meta.json"),
            r#"{"icon_url":"https://cdn.example.com/nhentai.png"}"#,
        )
        .unwrap();
        let entries = state.source_entries().unwrap();
        let nh = entries.iter().find(|s| s.id == "nhentai").unwrap();
        assert_eq!(
            nh.icon_url.as_deref(),
            Some("https://cdn.example.com/nhentai.png")
        );
        assert_eq!(nh.display_name.as_deref(), Some("NHentai"));
        assert!(nh.installed);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Config tanpa `ui` sama sekali → icon_url/display_name None (fallback UI).
    #[test]
    fn source_without_ui_section_has_no_icon() {
        let dir = std::env::temp_dir().join(format!("kuron-icon-empty-{}", std::process::id()));
        let state = state_with_ext_dir(&dir);
        std::fs::write(
            dir.join("polos-config.json"),
            r#"{"source":"polos","version":"0.1","baseUrl":"https://p.example"}"#,
        )
        .unwrap();
        let entries = state.source_entries().unwrap();
        let p = entries.iter().find(|s| s.id == "polos").unwrap();
        assert!(p.icon_url.is_none());
        assert!(p.display_name.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(feature = "live-tests")]
    #[test]
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
        let scfg = source_config_from_json(
            "areakomik",
            &cfg_file.base_url,
            scraper_sec,
            &cfg_file.default_language,
        )
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
        assert!(entries
            .iter()
            .any(|e| e.id == "nhentai" && !e.version.is_empty()));
    }
}
