//! Config-driven sources — port `informations/configs/*.json` + `assets/configs/`
//! mobile (44+1 file, dibundel di `src-tauri/resources/source-configs/`).
//! Struct toleran: field tak dikenal diabaikan; `scraper/selectors/searchForm`
//! penuh ditahan sebagai `serde_json::Value` untuk port lanjutan.

use std::{collections::HashMap, path::{Path, PathBuf}};

use serde::Deserialize;

use crate::core::AppError;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ApiSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, rename = "apiBase")]
    pub api_base: String,
    /// String (`"/x/{id}"`) atau map (`{"path": "/x", "params": {...}}`).
    #[serde(default)]
    pub endpoints: HashMap<String, serde_json::Value>,
    /// Blok bersarang ala MangaDex (`api.detail.chapters.endpoint`,
    /// `api.images.atHomeEndpoint`) — Value, dinavigasi per perlu.
    #[serde(default)]
    pub detail: serde_json::Value,
    #[serde(default)]
    pub images: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RateLimitSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, rename = "requestsPerMinute")]
    pub requests_per_minute: f64,
    #[serde(default, rename = "requestsPerSecond")]
    pub requests_per_second: f64,
    #[serde(default, rename = "minDelayMs")]
    pub min_delay_ms: u64,
}

impl RateLimitSection {
    /// Resolusi ala `GenericSourceFactory` mobile:
    /// minDelayMs → requestsPerSecond → requestsPerMinute.
    pub fn resolve_delay_ms(&self) -> u64 {
        if self.min_delay_ms > 0 {
            return self.min_delay_ms;
        }
        if self.requests_per_second > 0.0 {
            return (1000.0 / self.requests_per_second).ceil() as u64;
        }
        if self.requests_per_minute > 0.0 {
            return (60000.0 / self.requests_per_minute).ceil() as u64;
        }
        0
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RetrySection {
    #[serde(default, rename = "maxAttempts")]
    pub max_attempts: u32,
    #[serde(default, rename = "delayMs")]
    pub delay_ms: u64,
    #[serde(default, rename = "exponentialBackoff")]
    pub exponential_backoff: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NetworkSection {
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default, rename = "rateLimit")]
    pub rate_limit: Option<RateLimitSection>,
    #[serde(default)]
    pub retry: Option<RetrySection>,
}

/// Satu file `*-config.json` mobile (skema assets/ maupun informations/ —
/// keduanya punya `source/version/baseUrl`, berbeda di `api` vs `scraper`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SourceFile {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, rename = "baseUrl")]
    pub base_url: String,
    #[serde(default, rename = "defaultLanguage")]
    pub default_language: String,
    #[serde(default)]
    pub api: Option<ApiSection>,
    #[serde(default)]
    pub network: Option<NetworkSection>,
    #[serde(default)]
    pub scraper: Option<ScraperSection>,
    /// Navigasi tap-tag (`navigation.tagQueryMapping`) — Value per perlu.
    #[serde(default)]
    pub navigation: serde_json::Value,
    /// Definisi form cari per-sumber (`searchForm` config) — Value per perlu.
    #[serde(default, rename = "searchForm")]
    pub search_form: serde_json::Value,
    #[serde(default, rename = "assetHosts")]
    pub asset_hosts: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FieldSel {
    #[serde(default)]
    pub selector: String,
    #[serde(default, rename = "type")]
    pub sel_type: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub transform: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListCfg {
    #[serde(default)]
    pub container: String,
    #[serde(default)]
    pub fields: HashMap<String, FieldSel>,
    /// Blok `pagination` (`{"next": "#unext", "links": ...}`) — token cursor mobile.
    #[serde(default)]
    pub pagination: HashMap<String, String>,
}

/// Satu pola URL: string polos (`"detail": "/series/{id}/"`) atau map
/// (`{"url": ..., "inherits": ..., "list": {...}}`).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum UrlPattern {
    Url(String),
    Full {
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        inherits: Option<String>,
        #[serde(default)]
        list: Option<ListCfg>,
    },
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScraperSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, rename = "urlPatterns")]
    pub url_patterns: HashMap<String, UrlPattern>,
    /// `selectors` penuh (detail/chapters/reader) — Value, dinavigasi per perlu.
    #[serde(default)]
    pub selectors: serde_json::Value,
}

impl ScraperSection {
    /// Resolve pola + warisan `inherits` (rantaian, anti-siklus).
    pub fn resolve(&self, name: &str) -> Option<ResolvedPattern<'_>> {
        let mut url: Option<&str> = None;
        let mut list: Option<&ListCfg> = None;
        let mut current = Some(name);
        let mut guard = 0;
        while let Some(n) = current {
            guard += 1;
            if guard > 8 {
                break;
            }
            match self.url_patterns.get(n)? {
                UrlPattern::Url(u) => {
                    if url.is_none() {
                        url = Some(u);
                    }
                    break;
                }
                UrlPattern::Full { url: u, inherits, list: l } => {
                    if url.is_none() {
                        url = u.as_deref();
                    }
                    if list.is_none() {
                        list = l.as_ref();
                    }
                    if url.is_some() && list.is_some() {
                        break;
                    }
                    current = inherits.as_deref();
                    if current.is_none() {
                        break;
                    }
                }
            }
        }
        Some(ResolvedPattern { url, list })
    }
}

pub struct ResolvedPattern<'a> {
    pub url: Option<&'a str>,
    pub list: Option<&'a ListCfg>,
}

impl SourceFile {
    /// Template endpoint `api.endpoints[name]` dengan `{key}` terisi params.
    /// Fallback `None` bila endpoint tak ada (caller pakai path bawaan).
    pub fn endpoint(&self, name: &str, params: &[(&str, &str)]) -> Option<String> {
        let template = self.api.as_ref()?.endpoints.get(name)?;
        // Nilai string langsung, atau map {path, params}.
        let raw = if let Some(s) = template.as_str() {
            s.to_string()
        } else {
            let path = template.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let mut url = path.to_string();
            if let Some(obj) = template.get("params").and_then(|p| p.as_object()) {
                let q: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        format!("{}={}", k, v.as_str().unwrap_or(&v.to_string()))
                    })
                    .collect();
                if !q.is_empty() {
                    url.push('?');
                    url.push_str(&q.join("&"));
                }
            }
            url
        };
        let url = fill_url(&raw, params);
        if url.starts_with("http://") || url.starts_with("https://") {
            return Some(url);
        }
        let base = self.api.as_ref().map(|a| a.api_base.as_str()).unwrap_or("");
        let base = if base.is_empty() {
            self.base_url.clone()
        } else {
            base.to_string()
        };
        Some(format!("{base}{url}"))
    }

    pub fn min_delay_ms(&self) -> u64 {
        self.network
            .as_ref()
            .and_then(|n| n.rate_limit.as_ref())
            .map(|r| r.resolve_delay_ms())
            .unwrap_or(0)
    }
}

/// Isi template `{key}` ala `GenericUrlBuilder` mobile: nilai query di-encode,
/// param kosong yang berasal dari placeholder DIHAPUS dari query string
/// (bukan dikirim kosong — sebagian API 400), URL absolut tak di-prefix base.
pub fn fill_url(template: &str, params: &[(&str, &str)]) -> String {
    let mut url = template.to_string();
    let mut emptied: Vec<&str> = Vec::new();
    for (k, v) in params {
        let encoded = if *k == "page" || *k == "id" || *k == "contentId" {
            v.to_string()
        } else {
            encode_query(v)
        };
        if template.contains(&format!("{{{k}}}")) && encoded.is_empty() {
            emptied.push(k);
        }
        url = url.replace(&format!("{{{k}}}"), &encoded);
    }
    strip_empty_params(&url, &emptied)
}

pub(crate) fn encode_query(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn strip_empty_params(url: &str, only: &[&str]) -> String {
    let Some(q) = url.find('?') else {
        return url.to_string();
    };
    let (base, query) = url.split_at(q);
    let kept: Vec<&str> = query[1..]
        .split('&')
        .filter(|pair| {
            let mut it = pair.splitn(2, '=');
            let name = it.next().unwrap_or("");
            let empty = matches!(it.next(), Some(v) if v.is_empty());
            !(empty && only.contains(&name))
        })
        .collect();
    if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}

pub struct SourceConfigs {
    map: HashMap<String, SourceFile>,
}

impl SourceConfigs {
    /// Dir config bawaan bundle: HANYA `nhentai-config.json` (1:1 mobile —
    /// sumber lain wajib install via ekstensi, lihat `extension.rs`).
    pub fn bundled_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/source-configs")
    }

    pub fn load_dir(dir: &Path) -> Result<Self, AppError> {
        let mut map = HashMap::new();
        let entries = std::fs::read_dir(dir).map_err(|e| {
            AppError::Storage(format!("config dir {}: {e}", dir.display()))
        })?;
        for entry in entries.flatten() {
            let path = entry.path();
            // HANYA `*-config.json` — meta ikon (`*-meta.json`) bukan sumber.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.extension().and_then(|e| e.to_str()) != Some("json")
                || !name.ends_with("-config.json")
            {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let cfg: SourceFile = serde_json::from_str(&raw).map_err(|e| {
                AppError::Storage(format!("config {}: {e}", path.display()))
            })?;
            let key = if cfg.source.is_empty() {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .trim_end_matches("-config")
                    .to_string()
            } else {
                cfg.source.clone()
            };
            map.insert(key, cfg);
        }
        Ok(Self { map })
    }

    pub fn get(&self, source: &str) -> Option<&SourceFile> {
        self.map.get(source)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, SourceFile> {
        self.map.iter()
    }

    /// Gabung bawaan + ter-install; installed menang bila id sama (ala mobile:
    /// cache menimpa bundled, version-driven re-download).
    pub fn load_overlay(bundled_dir: &Path, installed_dir: &Path) -> Result<Self, AppError> {
        let mut map = Self::load_dir_opt(bundled_dir)?.map;
        let installed = Self::load_dir_opt(installed_dir)?;
        map.extend(installed.map);
        Ok(Self { map })
    }

    fn load_dir_opt(dir: &Path) -> Result<Self, AppError> {
        if !dir.exists() {
            return Ok(Self { map: HashMap::new() });
        }
        Self::load_dir(dir)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_nhentai_only_and_overlay_wins() {
        // Bundle: hanya nhentai (1:1 mobile).
        let bundled = SourceConfigs::load_dir(&SourceConfigs::bundled_dir()).unwrap();
        assert_eq!(bundled.len(), 1, "bundle wajib 1 config");
        let nh = bundled.get("nhentai").expect("nhentai bawaan");
        assert_eq!(nh.base_url, "https://nhentai.net");
        // Overlay: installed menimpa bundled bila id sama.
        let dir = std::env::temp_dir().join(format!("kuron-test-ov-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nhentai-config.json"),
            r#"{"source": "nhentai", "version": "9.9.9", "baseUrl": "https://mirror.example"}"#,
        )
        .unwrap();
        let merged =
            SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &dir).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged.get("nhentai").unwrap().version, "9.9.9");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fill_url_encodes_and_strips_empty() {
        // Encode query + drop param kosong (ala GenericUrlBuilder).
        let u = fill_url("/s?q={query}&sort={sort}&page={page}", &[("query", "a&b c"), ("sort", ""), ("page", "2")]);
        assert_eq!(u, "/s?q=a%26b+c&page=2", "{u}");
        // URL absolut tak diapa-apakan selain substitusi.
        let u = fill_url("https://x.example/{id}", &[("id", "1")]);
        assert_eq!(u, "https://x.example/1");
        // Tanpa query string: utuh.
        let u = fill_url("/g/{id}/", &[("id", "7")]);
        assert_eq!(u, "/g/7/");
    }

    #[test]
    fn rate_resolve_chain() {
        use serde_json::json;
        let r: RateLimitSection = serde_json::from_value(json!({"minDelayMs": 200})).unwrap();
        assert_eq!(r.resolve_delay_ms(), 200);
        let r: RateLimitSection =
            serde_json::from_value(json!({"requestsPerSecond": 2})).unwrap();
        assert_eq!(r.resolve_delay_ms(), 500);
        let r: RateLimitSection =
            serde_json::from_value(json!({"requestsPerMinute": 60})).unwrap();
        assert_eq!(r.resolve_delay_ms(), 1000);
        assert_eq!(RateLimitSection::default().resolve_delay_ms(), 0);
    }

    #[test]
    fn meta_files_ignored_by_loader() {
        let dir = std::env::temp_dir().join(format!("kuron-test-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x-config.json"), r#"{"source": "x", "baseUrl": "https://x.example"}"#).unwrap();
        std::fs::write(dir.join("x-meta.json"), r#"{"icon_url": "https://x.example/i.png"}"#).unwrap();
        let cfgs = SourceConfigs::load_dir(&dir).unwrap();
        assert_eq!(cfgs.len(), 1);
        assert!(cfgs.get("x").is_some());
        assert!(cfgs.get("x-meta").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_mobile_config_schemas() {
        // Dua varian skema mobile sebagai fixture.
        let dir = std::env::temp_dir().join(format!("kuron-test-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nhentai-config.json"),
            r#"{"source": "nhentai", "version": "1.0.10", "baseUrl": "https://nhentai.net",
                "api": {"enabled": true, "apiBase": "https://nhentai.net",
                        "endpoints": {"search": "/api/v2/search?query={query}&sort={sort}&page={page}"}},
                "assetHosts": {"image": "https://i.nhentai.net", "thumbnail": "https://t.nhentai.net"},
                "network": {"rateLimit": {"enabled": true, "minDelayMs": 200},
                            "retry": {"maxAttempts": 3, "delayMs": 1000, "exponentialBackoff": true}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("schale-config.json"),
            r#"{"source": "schale-network", "version": "1.0.0", "baseUrl": "https://api.schale.network",
                "api": {"enabled": true, "endpoints": {"search": {"path": "/books", "params": {"s": "{query}"}}}}}"#,
        )
        .unwrap();
        let cfgs = SourceConfigs::load_dir(&dir).unwrap();
        assert_eq!(cfgs.len(), 2);
        let nh = cfgs.get("nhentai").unwrap();
        let search = nh
            .endpoint("search", &[("query", "test"), ("sort", "popular"), ("page", "1")])
            .unwrap();
        assert!(search.contains("/api/v2/search?query=test"), "{search}");
        assert_eq!(nh.min_delay_ms(), 200);
        let sch = cfgs.get("schale-network").unwrap();
        let url = sch.endpoint("search", &[("query", "x")]).unwrap();
        assert!(url.contains("/books?s=x"), "{url}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
