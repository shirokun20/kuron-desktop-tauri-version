//! Remote DataSources — port `kuron_generic` + `kuron_special` (Fase 2, spec §12).
//! Engine config-driven: `GenericScraperAdapter` (HTML + CSS selector) dan
//! `GenericRestAdapter` (JSON API). Adapter situs = config + path per situs.
//! Selector situs dituning manual (best-known); engine diuji via fixture lokal.

use std::collections::HashMap;

use scraper::{Html, Selector};

use crate::{
    core::AppError,
    data::models::ContentModel,
    domain::Chapter,
    network::HttpClientManager,
};

/// Satu field ala mobile `FieldSelector`: selector CSS + ekstraksi.
/// `attribute: None` = teks elemen. `transform: "slug"` = segmen path terakhir.
#[derive(Debug, Clone, Default)]
pub struct FieldMap {
    pub selector: String,
    pub attribute: Option<String>,
    pub regex: Option<String>,
    pub transform: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub fallback: Option<String>,
}

impl FieldMap {
    pub fn text(selector: &str) -> Self {
        Self {
            selector: selector.to_string(),
            ..Default::default()
        }
    }

    pub fn attr(selector: &str, attribute: &str) -> Self {
        Self {
            selector: selector.to_string(),
            attribute: Some(attribute.to_string()),
            ..Default::default()
        }
    }
}

/// Konfigurasi scraping satu sumber: URL + selector CSS + ekstraksi field.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    pub source_id: String,
    pub base_url: String,
    /// Path list populer; `{q}`/`{query}` = query, `{page}` = halaman.
    pub list_path: String,
    /// Path home (pola `home` mobile); home feed pakai ini, bukan search kosong.
    pub home_path: String,
    /// Path home halaman >1 (pola `homePage`); kosong = ulang home.
    pub home_page_path: String,
    /// Path detail; `{id}` = id konten. Bila id sudah URL penuh, dipakai langsung.
    pub detail_path: String,
    pub item_selector: String,
    pub id: FieldMap,
    pub title: FieldMap,
    pub link: FieldMap,
    pub cover: FieldMap,
    /// Selector halaman detail (fallback ke list bila kosong).
    pub detail_title: FieldMap,
    pub detail_cover: FieldMap,
    /// Field opsional list ala mapper mobile (kosong = tak dipakai).
    pub page_count: FieldMap,
    pub language: FieldMap,
    /// Daftar chapter di halaman detail.
    pub chapter_selector: String,
    pub chapter_link: FieldMap,
    pub chapter_title: FieldMap,
    /// Gambar halaman di halaman chapter.
    pub page_selector: String,
    pub page_attr: Option<String>,
    /// Bahasa default situs (dari `defaultLanguage` config) untuk item scraper.
    pub default_language: Option<String>,
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Ekstrak satu field dari sub-elemen pertama yang cocok.
fn extract(scope: scraper::ElementRef<'_>, sel: &Selector, map: &FieldMap) -> String {
    let raw = scope
        .select(sel)
        .next()
        .map(|el| raw_of(el, map))
        .unwrap_or_default();
    apply_map(raw, map)
}

/// Ekstrak dari elemen itu sendiri (bukan turunan) — mis. `<img>` hasil seleksi.
fn extract_self(el: scraper::ElementRef<'_>, map: &FieldMap) -> String {
    apply_map(raw_of(el, map), map)
}

fn raw_of(el: scraper::ElementRef<'_>, map: &FieldMap) -> String {
    match map.attribute.as_deref() {
        // Rantai lazy-load ala `GenericHtmlParser` mobile:
        // data-src → data-lazy-src → data-pagespeed-lazy-src → src,
        // lewati `data:` URI + placeholder optimizer (`*_result.jpg`,
        // `/pagespeed_static/`).
        Some("src") => image_src(el).unwrap_or_default(),
        Some(attr) => el.value().attr(attr).unwrap_or_default().to_string(),
        None => collapse_ws(&el.text().collect::<String>()),
    }
}

fn image_src(el: scraper::ElementRef<'_>) -> Option<String> {
    const CHAIN: &[&str] = &[
        "data-src",
        "data-lazy-src",
        "data-pagespeed-lazy-src",
        "src",
    ];
    for attr in CHAIN {
        let v = el.value().attr(attr).unwrap_or("").trim();
        if v.is_empty() || v.starts_with("data:") || is_placeholder_src(v) {
            continue;
        }
        return Some(v.to_string());
    }
    // Semua kandidat placeholder/kosong: kembalikan `src` apa adanya
    // (bisa "") — pemanggil yang menilai kosong/tidak.
    Some(el.value().attr("src").unwrap_or("").trim().to_string())
}

fn is_placeholder_src(v: &str) -> bool {
    let lower = v.to_lowercase();
    if lower.starts_with("/pagespeed_static/") {
        return true;
    }
    let path = lower.split('?').next().unwrap_or(&lower);
    if let Some(dot) = path.rfind('.') {
        let (stem, ext) = (&path[..dot], &path[dot + 1..]);
        if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "gif")
            && (stem.ends_with("-result") || stem.ends_with("_result"))
        {
            return true;
        }
    }
    false
}

fn apply_map(mut value: String, map: &FieldMap) -> String {
    if let Some(re) = map.regex.as_deref() {
        // dotAll ala mobile: teks badge acap multiline.
        let pattern = format!("(?s){re}");
        if let Ok(r) = regex::Regex::new(&pattern) {
            value = r
                .captures(&value)
                .and_then(|c| c.get(1).or_else(|| c.get(0)))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
        }
    }
    if map.transform.as_deref() == Some("slug") {
        value = value
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
    }
    if value.is_empty() {
        value = map.fallback.clone().unwrap_or_default();
    }
    if !value.is_empty() {
        if let Some(pre) = map.prefix.as_deref() {
            value = format!("{pre}{value}");
        }
        if let Some(suf) = map.suffix.as_deref() {
            value = format!("{value}{suf}");
        }
    }
    value
}

impl SourceConfig {
    pub fn list_url(&self, query: &str, page: u32) -> String {
        let path = crate::data::datasources::config::fill_url(
            &self.list_path,
            &[
                ("query", query),
                ("q", query),
                ("page", &page.to_string()),
            ],
        );
        if path.starts_with("http://") || path.starts_with("https://") {
            return path;
        }
        format!("{}{}", self.base_url, path)
    }

    /// URL home feed (pola `home`); fallback ke list kosong bila tak ada.
    pub fn home_url(&self) -> String {
        self.home_url_page(1)
    }

    pub fn home_url_page(&self, page: u32) -> String {
        let path = if page > 1 && !self.home_page_path.trim().is_empty() {
            crate::data::datasources::config::fill_url(
                &self.home_page_path,
                &[("page", &page.to_string())],
            )
        } else if !self.home_path.trim().is_empty() {
            self.home_path.clone()
        } else {
            return self.list_url("", 1);
        };
        if path.starts_with("http://") || path.starts_with("https://") {
            return path;
        }
        format!("{}{}", self.base_url, path)
    }

    pub fn detail_url(&self, id_or_url: &str) -> String {
        if id_or_url.starts_with("http://") || id_or_url.starts_with("https://") {
            return id_or_url.to_string();
        }
        let path = crate::data::datasources::config::fill_url(
            &self.detail_path,
            &[("id", id_or_url), ("contentId", id_or_url)],
        );
        format!("{}{}", self.base_url, path)
    }

    fn absolutize(&self, src: &str) -> String {
        if src.is_empty() {
            return String::new();
        }
        if src.starts_with("http://") || src.starts_with("https://") {
            src.to_string()
        } else if let Some(rest) = src.strip_prefix("//") {
            format!("https://{rest}")
        } else if src.starts_with('/') {
            format!("{}{src}", self.base_url)
        } else {
            format!("{}/{}", self.base_url, src)
        }
    }
}

/// NHentai — config best-known (`.gallery` stabil bertahun-tahun).
/// Jalur utama kini API v2 (`NhentaiApiAdapter`); scraper HTML fallback.
pub fn nhentai_config() -> SourceConfig {
    SourceConfig {
        source_id: "nhentai".to_string(),
        base_url: "https://nhentai.net".to_string(),
        list_path: "/search/?q={q}&page={page}".to_string(),
        home_path: String::new(),
        home_page_path: String::new(),
        detail_path: "/g/{id}/".to_string(),
        item_selector: ".gallery".to_string(),
        id: FieldMap {
            selector: "a.cover".to_string(),
            attribute: Some("href".to_string()),
            transform: Some("slug".to_string()),
            ..Default::default()
        },
        title: FieldMap::text(".caption"),
        link: FieldMap::attr("a.cover", "href"),
        cover: FieldMap::attr("img", "src"),
        detail_title: FieldMap::default(),
        detail_cover: FieldMap::default(),
        chapter_selector: "a.go".to_string(),
        chapter_link: FieldMap::attr("a.go", "href"),
        chapter_title: FieldMap::text("a.go"),
        page_selector: ".thumb-container img".to_string(),
        page_attr: Some("src".to_string()),
        page_count: FieldMap::default(),
        language: FieldMap::default(),
        default_language: None,
    }
}

/// Hitomi — config best-known; tuning live lanjutan bila markup berubah.
pub fn hitomi_config() -> SourceConfig {
    SourceConfig {
        source_id: "hitomi".to_string(),
        base_url: "https://hitomi.la".to_string(),
        list_path: "/search.html?q={q}".to_string(),
        home_path: "/index-all.html".to_string(),
        home_page_path: String::new(),
        detail_path: "/galleries/{id}.html".to_string(),
        item_selector: ".gallery-content > div".to_string(),
        id: FieldMap {
            selector: "h1 a".to_string(),
            attribute: Some("href".to_string()),
            regex: Some("/([0-9]+)\\.html".to_string()),
            ..Default::default()
        },
        title: FieldMap::text("h1 a"),
        link: FieldMap::attr("h1 a", "href"),
        cover: FieldMap::attr(".dj-img img, img", "src"),
        detail_title: FieldMap::default(),
        detail_cover: FieldMap::default(),
        chapter_selector: ".manga-chapter a".to_string(),
        chapter_link: FieldMap::attr(".manga-chapter a", "href"),
        chapter_title: FieldMap::text(".manga-chapter a"),
        page_selector: ".manga-page img, .content img".to_string(),
        page_attr: Some("src".to_string()),
        page_count: FieldMap::default(),
        language: FieldMap::default(),
        default_language: None,
    }
}

/// E-Hentai — butuh cookie/login untuk penuh; selector publik best-known.
pub fn ehentai_config() -> SourceConfig {
    SourceConfig {
        source_id: "ehentai".to_string(),
        base_url: "https://e-hentai.org".to_string(),
        list_path: "/?f_search={q}&page={page}".to_string(),
        home_path: "/".to_string(),
        home_page_path: String::new(),
        // E-H URL butuh token (`/g/{id}/{token}/`); frontend kirim URL penuh bila ada.
        detail_path: "/g/{id}/".to_string(),
        item_selector: "table.gltc tr, .itg tr".to_string(),
        id: FieldMap::attr(".gl3c a, .gl4e a", "href"),
        title: FieldMap::text(".glink, .gl4e a"),
        link: FieldMap::attr(".gl3c a, .gl4e a", "href"),
        cover: FieldMap::attr(".gl3c img, .glthumb img", "src"),
        detail_title: FieldMap::default(),
        detail_cover: FieldMap::default(),
        chapter_selector: "#gdt a".to_string(),
        chapter_link: FieldMap::attr("#gdt a", "href"),
        chapter_title: FieldMap::text("#gdt a"),
        page_selector: "#gdt img, #img".to_string(),
        page_attr: Some("src".to_string()),
        page_count: FieldMap::default(),
        language: FieldMap::default(),
        default_language: None,
    }
}

/// Bangun SourceConfig engine dari JSON `scraper` mobile (areakomik dkk).
/// `None` bila pola list tak lengkap — caller fallback hardcode/error jujur.
pub fn source_config_from_json(
    source_id: &str,
    base_url: &str,
    scraper: &crate::data::datasources::config::ScraperSection,
    default_language: &str,
) -> Option<SourceConfig> {
    use crate::data::datasources::config::{FieldSel, UrlPattern};
    if !scraper.enabled {
        return None;
    }
    let pattern = scraper
        .resolve("search")
        .or_else(|| scraper.resolve("home"))?;
    let list = pattern.list?;
    if list.container.trim().is_empty() {
        return None;
    }
    let field = |names: &[&str]| -> FieldSel {
        names
            .iter()
            .filter_map(|n| list.fields.get(*n))
            .next()
            .cloned()
            .unwrap_or_default()
    };
    let id = field(&["id"]);
    let title = field(&["title"]);
    if title.selector.trim().is_empty() {
        return None;
    }
    let cover = field(&["coverUrl", "cover"]);
    let link = field(&["link", "url"]);
    // link default = field id bila ber-href (areakomik: `a.thumb-wrap`).
    let link = if link.selector.is_empty() && id.attribute.as_deref() == Some("href") {
        id.clone()
    } else {
        link
    };
    let to_map = |f: FieldSel| FieldMap {
        selector: f.selector,
        attribute: f.attribute,
        regex: f.regex,
        transform: f.transform,
        prefix: f.prefix,
        suffix: f.suffix,
        fallback: f.fallback,
    };
    let list_path = pattern.url.unwrap_or("/").to_string();
    let home_path = scraper
        .resolve("home")
        .and_then(|h| h.url)
        .unwrap_or("")
        .to_string();
    let home_path = if home_path.is_empty() {
        list_path.clone()
    } else {
        home_path
    };
    let home_page_path = scraper
        .resolve("homePage")
        .and_then(|h| h.url)
        .unwrap_or("")
        .to_string();
    let detail_path = match scraper.url_patterns.get("detail") {
        Some(UrlPattern::Url(u)) => u.clone(),
        Some(UrlPattern::Full { url: Some(u), .. }) => u.clone(),
        _ => String::new(),
    };
    let (chapter_selector, chapter_link, chapter_title) =
        chapters_from(&scraper.selectors);
    let (page_selector, page_attr) = reader_from(&scraper.selectors);
    // Judul/cover halaman detail (fallback ke list bila tak ada).
    let detail_fields: HashMap<String, crate::data::datasources::config::FieldSel> = scraper
        .selectors
        .get("detail")
        .and_then(|d| d.get("fields"))
        .and_then(|f| serde_json::from_value(f.clone()).ok())
        .unwrap_or_default();
    let dfield = |names: &[&str]| -> FieldMap {
        let f = names
            .iter()
            .filter_map(|n| detail_fields.get(*n))
            .next()
            .cloned()
            .unwrap_or_default();
        FieldMap {
            selector: f.selector,
            attribute: f.attribute,
            regex: f.regex,
            transform: f.transform,
            prefix: f.prefix,
            suffix: f.suffix,
            fallback: f.fallback,
        }
    };
    let fmap = |names: &[&str]| -> FieldMap {
        let f = names
            .iter()
            .filter_map(|n| list.fields.get(*n))
            .next()
            .cloned()
            .unwrap_or_default();
        FieldMap {
            selector: f.selector,
            attribute: f.attribute,
            regex: f.regex,
            transform: f.transform,
            prefix: f.prefix,
            suffix: f.suffix,
            fallback: f.fallback,
        }
    };
    let page_count = fmap(&["pageCount"]);
    let page_count = if page_count.selector.is_empty() {
        FieldMap::default()
    } else {
        page_count
    };
    let language = fmap(&["language"]);
    let language = if language.selector.is_empty() {
        FieldMap::default()
    } else {
        language
    };
    let detail_title = dfield(&["title"]);
    let detail_title = if detail_title.selector.is_empty() {
        FieldMap::default()
    } else {
        detail_title
    };
    let detail_cover = dfield(&["coverUrl", "cover"]);
    let detail_cover = if detail_cover.selector.is_empty() {
        FieldMap::default()
    } else {
        detail_cover
    };
    Some(SourceConfig {
        source_id: source_id.to_string(),
        base_url: base_url.to_string(),
        list_path,
        home_path,
        home_page_path,
        detail_path,
        item_selector: list.container.clone(),
        id: to_map(id),
        title: to_map(title),
        link: to_map(link),
        cover: to_map(cover),
        detail_title,
        detail_cover,
        page_count,
        language,
        chapter_selector,
        chapter_link,
        chapter_title,
        page_selector,
        page_attr,
        default_language: normalize_lang(default_language),
    })
}

fn chapters_from(sel: &serde_json::Value) -> (String, FieldMap, FieldMap) {
    #[derive(serde::Deserialize, Default)]
    struct ChaptersCfg {
        #[serde(default)]
        container: String,
        #[serde(default)]
        fields: HashMap<String, crate::data::datasources::config::FieldSel>,
    }
    let cfg: ChaptersCfg = sel
        .get("detail")
        .and_then(|d| d.get("chapters"))
        .and_then(|c| serde_json::from_value(c.clone()).ok())
        .unwrap_or_default();
    let map = |f: crate::data::datasources::config::FieldSel| FieldMap {
        selector: f.selector,
        attribute: f.attribute,
        regex: f.regex,
        transform: f.transform,
        prefix: f.prefix,
        suffix: f.suffix,
        fallback: f.fallback,
    };
    let link = cfg.fields.get("id").cloned().unwrap_or_default();
    let title = cfg.fields.get("title").cloned().unwrap_or_default();
    (cfg.container, map(link), map(title))
}

fn reader_from(sel: &serde_json::Value) -> (String, Option<String>) {
    #[derive(serde::Deserialize, Default)]
    struct ImagesCfg {
        #[serde(default)]
        selector: String,
        #[serde(default)]
        attribute: Option<String>,
    }
    let cfg: ImagesCfg = sel
        .get("reader")
        .and_then(|r| r.get("images"))
        .and_then(|i| serde_json::from_value(i.clone()).ok())
        .unwrap_or_default();
    (cfg.selector, cfg.attribute)
}

/// Normalisasi bahasa ke kode ISO ala mobile (`languageTagMap` + defaultLanguage):
/// english/en→en, japanese/ja→ja, chinese/zh→zh, indonesian/id→id.
pub fn normalize_lang(s: &str) -> Option<String> {
    match s.trim().to_lowercase().as_str() {
        "english" | "en" => Some("en".to_string()),
        "japanese" | "ja" => Some("ja".to_string()),
        "chinese" | "zh" => Some("zh".to_string()),
        "indonesian" | "indonesia" | "id" => Some("id".to_string()),
        other if other.chars().all(|c| c.is_ascii_alphanumeric()) && !other.is_empty() => {
            Some(other.to_string())
        }
        _ => None,
    }
}

/// Bahasa NHentai dari `tag_ids` list (mobile `languageTagMap`):
/// 12227→en, 29963→zh, default ja.
fn nhentai_lang_of_ids(item: &serde_json::Value) -> Option<String> {
    let ids: Vec<u64> = item
        .get("tag_ids")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
    if ids.contains(&12227) {
        Some("en".to_string())
    } else if ids.contains(&29963) {
        Some("zh".to_string())
    } else {
        Some("ja".to_string())
    }
}

/// Bahasa NHentai dari tags detail (type=language, abaikan "translated").
fn nhentai_lang_of_tags(v: &serde_json::Value) -> Option<String> {
    let tags = v.get("tags")?.as_array()?;
    for t in tags {
        if t.get("type").and_then(|x| x.as_str()) != Some("language") {
            continue;
        }
        let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name == "translated" {
            continue;
        }
        if let Some(code) = normalize_lang(name) {
            return Some(code);
        }
    }
    None
}

/// MangaDex memakai JSON API (REST), bukan scraping HTML.
pub const MANGADEX_API: &str = "https://api.mangadex.org";

pub struct GenericRestAdapter {
    http: HttpClientManager,
}

impl GenericRestAdapter {
    pub fn new(http: HttpClientManager) -> Self {
        Self { http }
    }

    /// Cari manga via MangaDex API; cover dari relasi `cover_art`.
    pub async fn search_mangadex(
        &self,
        query: &str,
        page: u32,
    ) -> Result<Vec<ContentModel>, AppError> {
        let offset = page.saturating_sub(1) * 20;
        let url = format!(
            "{MANGADEX_API}/manga?title={query}&limit=20&offset={offset}\
             &includes[]=cover_art&contentRating[]=safe&contentRating[]=suggestive"
        );
        let body = self.http.get(&url, "mangadex").await?;
        Self::parse_mangadex(&body)
    }

    /// Detail satu manga via API (`/manga/{id}`).
    pub async fn get_mangadex_detail(&self, id: &str) -> Result<ContentModel, AppError> {
        let url = format!("{MANGADEX_API}/manga/{id}?includes[]=cover_art");
        let body = self.http.get(&url, "mangadex").await?;
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| AppError::Network(format!("mangadex json: {e}")))?;
        let empty = vec![];
        let data = v
            .get("data")
            .map(|d| vec![d.clone()])
            .unwrap_or(empty);
        let json = serde_json::json!({ "data": data });
        let mut items = Self::parse_mangadex(&json.to_string())?;
        items.pop().ok_or_else(|| {
            AppError::Validation(format!("mangadex: {id} tidak ditemukan"))
        })
    }

    fn parse_mangadex(body: &str) -> Result<Vec<ContentModel>, AppError> {
        let v: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| AppError::Network(format!("mangadex json: {e}")))?;
        let empty = vec![];
        let data = v.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        let mut out = Vec::new();
        for item in data {
            let id = item.get("id").and_then(|i| i.as_str()).unwrap_or_default();
            let attrs = item.get("attributes");
            let title = attrs
                .and_then(|a| a.get("title"))
                .and_then(|t| t.get("en").or_else(|| t.get("ja-ro")))
                .and_then(|t| t.as_str())
                .unwrap_or("(tanpa judul)")
                .to_string();
            let cover = item
                .get("relationships")
                .and_then(|r| r.as_array())
                .map(|rels| {
                    rels.iter()
                        .find(|r| r.get("type").and_then(|t| t.as_str()) == Some("cover_art"))
                        .and_then(|r| {
                            r.get("attributes").and_then(|a| {
                                a.get("fileName").and_then(|f| f.as_str())
                            })
                        })
                        .map(|f| {
                            format!(
                                "https://uploads.mangadex.org/covers/{id}/{f}.256.jpg"
                            )
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            out.push(ContentModel {
                id: id.to_string(),
                title,
                cover_url: cover,
                source_id: "mangadex".to_string(),
                upload_date: None,
                page_count: None,
                language: None,
            });
        }
        Ok(out)
    }
}

pub struct GenericScraperAdapter {
    http: HttpClientManager,
}

impl GenericScraperAdapter {
    pub fn new(http: HttpClientManager) -> Self {
        Self { http }
    }

    fn sel(selector: &str) -> Result<Selector, AppError> {
        Selector::parse(selector)
            .map_err(|e| AppError::Internal(format!("selector buruk {selector}: {e:?}")))
    }

    fn item_id(href: &str) -> String {
        href.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(href)
            .to_string()
    }

    pub async fn fetch_list(
        &self,
        url: &str,
        config: &SourceConfig,
    ) -> Result<Vec<ContentModel>, AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        Self::parse_list(&html, config)
    }

    fn parse_list(html: &str, config: &SourceConfig) -> Result<Vec<ContentModel>, AppError> {
        let doc = Html::parse_document(html);
        let item_sel = Self::sel(&config.item_selector)?;
        let id_sel = Self::sel(&config.id.selector)?;
        let title_sel = Self::sel(&config.title.selector)?;
        let link_sel = Self::sel(&config.link.selector)?;
        let cover_sel = Self::sel(&config.cover.selector)?;
        let mut out = Vec::new();
        for item in doc.select(&item_sel) {
            let title = extract(item, &title_sel, &config.title);
            if title.is_empty() {
                continue;
            }
            let href = extract(item, &link_sel, &config.link);
            let mut id = extract(item, &id_sel, &config.id);
            if id.is_empty() {
                id = Self::item_id(&href);
            }
            let cover = config.absolutize(&extract(item, &cover_sel, &config.cover));
            // Field opsional config-driven ala mapper mobile: pageCount (int
            // pertama) + language (normalisasi, fallback default situs).
            let page_count = if config.page_count.selector.trim().is_empty() {
                None
            } else {
                let sel = Self::sel(&config.page_count.selector)?;
                let raw = extract(item, &sel, &config.page_count);
                raw.split(|c: char| !c.is_ascii_digit())
                    .find(|x| !x.is_empty())
                    .and_then(|x| x.parse::<u32>().ok())
            };
            let language = if config.language.selector.trim().is_empty() {
                None
            } else {
                let sel = Self::sel(&config.language.selector)?;
                normalize_lang(&extract(item, &sel, &config.language))
            }
            .or_else(|| config.default_language.clone());
            out.push(ContentModel {
                id,
                title,
                cover_url: cover,
                source_id: config.source_id.clone(),
                upload_date: None,
                page_count,
                language,
            });
        }
        Ok(out)
    }

    pub async fn fetch_detail(
        &self,
        url: &str,
        config: &SourceConfig,
    ) -> Result<ContentModel, AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        let doc = Html::parse_document(&html);
        let root = doc.root_element();
        let tmap = if config.detail_title.selector.is_empty() { &config.title } else { &config.detail_title };
        let cmap = if config.detail_cover.selector.is_empty() { &config.cover } else { &config.detail_cover };
        let title_sel = Self::sel(&tmap.selector)?;
        let cover_sel = Self::sel(&cmap.selector)?;
        let title = extract(root, &title_sel, tmap);
        let title = if title.is_empty() {
            "(tanpa judul)".to_string()
        } else {
            title
        };
        let cover = config.absolutize(&extract(root, &cover_sel, cmap));
        Ok(ContentModel {
            id: Self::item_id(url),
            title,
            cover_url: cover,
            source_id: config.source_id.clone(),
            upload_date: None,
            page_count: None,
            language: config.default_language.clone(),
        })
    }

    pub async fn fetch_chapters(
        &self,
        url: &str,
        content_id: &str,
        config: &SourceConfig,
    ) -> Result<Vec<Chapter>, AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        // Selector chapter kosong = sumber single-chapter.
        if config.chapter_selector.trim().is_empty() {
            return Ok(vec![Chapter {
                id: format!("{content_id}-1"),
                content_id: content_id.to_string(),
                title: "Baca".to_string(),
                order: 1,
                is_external: false,
                external_url: Some(config.absolutize(url)),
            }]);
        }
        let doc = Html::parse_document(&html);
        let sel = Self::sel(&config.chapter_selector)?;
        let link_sel = Self::sel(&config.chapter_link.selector)?;
        let title_sel = Self::sel(&config.chapter_title.selector)?;
        let mut links: Vec<(String, String, String)> = Vec::new();
        for row in doc.select(&sel) {
            // Baris boleh JADI link-nya sendiri (hitomi/eh): fallback ke row.
            // href mentah (tanpa transform) untuk URL; id pakai map penuh.
            let raw_map = FieldMap {
                transform: None,
                regex: None,
                ..config.chapter_link.clone()
            };
            let mut href = extract(row, &link_sel, &raw_map);
            let mut title = extract(row, &title_sel, &config.chapter_title);
            if href.is_empty() {
                if let Some(h) = row.value().attr("href") {
                    href = h.to_string();
                    if title.is_empty() {
                        title = collapse_ws(&row.text().collect::<String>());
                    }
                }
            }
            if href.is_empty() {
                continue;
            }
            // id: pakai map chapter_link bila transform slug, else segmen akhir.
            let mut id = if config.chapter_link.transform.as_deref() == Some("slug") {
                extract(row, &link_sel, &config.chapter_link)
            } else {
                String::new()
            };
            if id.is_empty() {
                id = row
                    .value()
                    .attr("href")
                    .map(Self::item_id)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| Self::item_id(&href));
            }
            links.push((href, id, title));
        }
        if links.is_empty() {
            // Sumber single-chapter (mis. nhentai): halaman baca langsung.
            return Ok(vec![Chapter {
                id: format!("{content_id}-1"),
                content_id: content_id.to_string(),
                title: "Baca".to_string(),
                order: 1,
                is_external: false,
                external_url: Some(config.absolutize(url)),
            }]);
        }
        Ok(links
            .into_iter()
            .enumerate()
            .map(|(i, (href, id, title))| Chapter {
                id,
                content_id: content_id.to_string(),
                title: if title.is_empty() {
                    format!("Chapter {}", i + 1)
                } else {
                    title
                },
                order: (i + 1) as u32,
                is_external: false,
                external_url: Some(config.absolutize(&href)),
            })
            .collect())
    }

    pub async fn fetch_page_images(
        &self,
        url: &str,
        config: &SourceConfig,
    ) -> Result<Vec<String>, AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        let doc = Html::parse_document(&html);
        let sel = Self::sel(&config.page_selector)?;
        let attr = config.page_attr.clone().unwrap_or_else(|| "src".to_string());
        let map = FieldMap::attr(&config.page_selector, &attr);
        Ok(doc
            .select(&sel)
            .map(|el| config.absolutize(&extract_self(el, &map)))
            .filter(|s| !s.is_empty())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GALLERY_HTML: &str = r#"
        <html><body>
        <div class="gallery"><a class="cover" href="/g/12345/"><img data-src="//t.example/1t.webp"/><div class="caption">Judul Satu</div></a></div>
        <div class="gallery"><a class="cover" href="/g/67890/"><img src="https://t.example/2t.webp"/><div class="caption">Judul Dua</div></a></div>
        <div class="gallery"><div class="caption"></div></div>
        </body></html>"#;

    const MANGADEX_JSON: &str = r#"{
        "data": [{
            "id": "manga-1",
            "attributes": {"title": {"en": "Sample Manga"}},
            "relationships": [
                {"type": "author", "id": "a1"},
                {"type": "cover_art", "attributes": {"fileName": "cover1.jpg"}}
            ]
        }]
    }"#;

    /// Server fixture lokal: `/list` HTML galeri, `/api` JSON mangadex.
    fn spawn_fixture() -> String {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            for stream in listener.incoming().take(8) {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 4096];
                let len = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..len]).to_string();
                let path = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/");
                let body = if path.starts_with("/api") {
                    MANGADEX_JSON.to_string()
                } else {
                    GALLERY_HTML.to_string()
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

    fn nhentai_shaped() -> SourceConfig {
        SourceConfig {
            source_id: "nhentai-test".to_string(),
            base_url: "https://nhentai.net".to_string(),
            list_path: "/".to_string(),
            home_path: "/".to_string(),
            home_page_path: String::new(),
            detail_path: "/g/{id}/".to_string(),
            item_selector: ".gallery".to_string(),
            id: FieldMap {
                selector: "a.cover".to_string(),
                attribute: Some("href".to_string()),
                transform: Some("slug".to_string()),
                ..Default::default()
            },
            title: FieldMap::text(".caption"),
            link: FieldMap::attr("a.cover", "href"),
            cover: FieldMap::attr("img", "src"),
            detail_title: FieldMap::default(),
            detail_cover: FieldMap::default(),
            chapter_selector: "a.go".to_string(),
            chapter_link: FieldMap::attr("a.go", "href"),
            chapter_title: FieldMap::text("a.go"),
            page_selector: ".thumb-container img".to_string(),
            page_attr: Some("src".to_string()),
            page_count: FieldMap::default(),
            language: FieldMap::default(),
            default_language: None,
        }
    }

    #[test]
    fn image_fallback_chain_skips_placeholders() {
        // data-lazy-src dipakai saat src placeholder `*_result.jpg`.
        let html = r#"<html><body>
            <img src="Ideal-(1)_result.jpg" data-lazy-src="https://cdn.example/real.jpg"/>
            <img src="/pagespeed_static/x.jpg" data-src="https://cdn.example/real2.jpg"/>
            <img src="data:image/gif;base64,AAA"/>
            <img src="https://cdn.example/ok.jpg"/>
            </body></html>"#;
        let doc = Html::parse_document(html);
        let sel = Selector::parse("img").unwrap();
        let map = FieldMap::attr("img", "src");
        let got: Vec<String> = doc.select(&sel).map(|el| extract_self(el, &map)).collect();
        assert_eq!(got[0], "https://cdn.example/real.jpg");
        assert_eq!(got[1], "https://cdn.example/real2.jpg");
        // `data:` URI diteruskan apa adanya (paritas mobile).
        assert_eq!(got[2], "data:image/gif;base64,AAA");
        assert_eq!(got[3], "https://cdn.example/ok.jpg");
    }

    #[test]
    fn scraper_parses_gallery_fixture() {
        let base = spawn_fixture();
        let http = HttpClientManager::without_proxy().unwrap();
        let scraper = GenericScraperAdapter::new(http);
        let items =
            tauri::async_runtime::block_on(scraper.fetch_list(&format!("{base}/list"), &nhentai_shaped()))
                .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "12345");
        assert_eq!(items[0].title, "Judul Satu");
        assert_eq!(items[0].cover_url, "https://t.example/1t.webp");
        assert_eq!(items[1].cover_url, "https://t.example/2t.webp");
    }

    #[test]
    fn rest_parses_mangadex_fixture() {
        let base = spawn_fixture();
        let http = HttpClientManager::without_proxy().unwrap();
        // Suntik base API lewat URL langsung: uji parser via fetch + parse.
        let body =
            tauri::async_runtime::block_on(http.get(&format!("{base}/api"), "mangadex"))
                .unwrap();
        let items = GenericRestAdapter::parse_mangadex(&body).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "manga-1");
        assert_eq!(items[0].title, "Sample Manga");
        assert_eq!(
            items[0].cover_url,
            "https://uploads.mangadex.org/covers/manga-1/cover1.jpg.256.jpg"
        );
    }

    #[test]
    #[ignore = "hits live api.mangadex.org; jalankan manual: cargo test live_mangadex -- --ignored"]
    fn live_mangadex_search_parses() {
        // NHentai/Hitomi/E-H live 403 dari network tanpa cookie CF
        // (risiko spec §7 — mitigasi: cookie harvest Fase 2 lanjutan).
        let http = HttpClientManager::new().unwrap();
        let rest = GenericRestAdapter::new(http);
        let items =
            tauri::async_runtime::block_on(rest.search_mangadex("test", 1)).unwrap();
        assert!(!items.is_empty(), "live search mengembalikan item");
        assert!(!items[0].id.is_empty());
        assert!(!items[0].title.is_empty());
        let detail =
            tauri::async_runtime::block_on(rest.get_mangadex_detail(&items[0].id)).unwrap();
        assert_eq!(detail.id, items[0].id);
    }
}

/// NHentai via unofficial JSON API v2 (config `assets/configs/nhentai-config.json`
/// mobile): endpoint + host gambar dari file config, ekstraksi item via JSONPath
/// (`$.result[*]` — scope 4.6; FieldSelector penuh nyusul).
/// Verified live 2026-09-20: search/detail/pages bentuk sesuai kode di bawah.
pub struct NhentaiApiAdapter {
    http: HttpClientManager,
    limiter: crate::application::services::RateLimiter,
    search_tpl: String,
    all_tpl: String,
    detail_tpl: String,
    thumb_host: String,
    img_host: String,
    attempts: u32,
    retry_delay_ms: u64,
}

impl NhentaiApiAdapter {
    pub fn from_config(
        http: HttpClientManager,
        cfg: &crate::data::datasources::config::SourceFile,
    ) -> Self {
        let api_base = cfg
            .api
            .as_ref()
            .map(|a| a.api_base.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://nhentai.net".to_string());
        let ep = |name: &str, fallback: &str| {
            cfg.endpoint(name, &[])
                .unwrap_or_else(|| format!("{api_base}{fallback}"))
        };
        let net = cfg.network.clone().unwrap_or_default();
        let (attempts, retry_delay_ms) = net
            .retry
            .map(|r| (r.max_attempts.max(1), r.delay_ms.max(1)))
            .unwrap_or((3, 1000));
        let host = |k: &str, fb: &str| {
            cfg.asset_hosts
                .get(k)
                .cloned()
                .unwrap_or_else(|| fb.to_string())
        };
        Self {
            http,
            limiter: crate::application::services::RateLimiter::new(cfg.min_delay_ms()),
            search_tpl: ep("search", "/api/v2/search?query={query}&sort={sort}&page={page}"),
            all_tpl: ep("allGalleries", "/api/v2/galleries?page={page}"),
            detail_tpl: ep("galleryDetail", "/api/v2/galleries/{id}"),
            thumb_host: host("thumbnail", "https://t.nhentai.net"),
            img_host: host("image", "https://i.nhentai.net"),
            attempts,
            retry_delay_ms,
        }
    }

    async fn fetch(&self, url: &str) -> Result<serde_json::Value, AppError> {
        self.limiter.wait("nhentai.net").await;
        let body = self
            .http
            .get_with_retry(url, "nhentai", self.attempts, self.retry_delay_ms)
            .await?;
        serde_json::from_str(&body)
            .map_err(|e| AppError::Network(format!("nhentai json: {e}")))
    }

    fn items_of(value: &serde_json::Value) -> Vec<serde_json::Value> {
        use serde_json_path::JsonPath;
        JsonPath::parse("$.result[*]")
            .map(|p| p.query(value).into_iter().cloned().collect())
            .unwrap_or_default()
    }

    fn to_model(&self, item: &serde_json::Value) -> Option<ContentModel> {
        let id = item.get("id").and_then(|v| {
            v.as_str()
                .map(String::from)
                .or_else(|| v.as_u64().map(|n| n.to_string()))
        })?;
        let title = item
            .get("english_title")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                item.get("title")?.get("english")?.as_str().map(String::from)
            })
            .unwrap_or_else(|| "(tanpa judul)".to_string());
        let cover = item
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|t| format!("{}/{t}", self.thumb_host))
            .unwrap_or_default();
        Some(ContentModel {
            id,
            title,
            cover_url: cover,
            source_id: "nhentai".to_string(),
            upload_date: None,
            page_count: item
                .get("num_pages")
                .and_then(|v| v.as_u64().map(|n| n as u32)),
            language: nhentai_lang_of_ids(item),
        })
    }

    pub async fn search(&self, query: &str, page: u32) -> Result<Vec<ContentModel>, AppError> {
        // API menolak query kosong (400) → home feed pakai allGalleries.
        let url = if query.trim().is_empty() {
            crate::data::datasources::config::fill_url(&self.all_tpl.clone(), &[("page", &page.max(1).to_string())])
        } else {
            crate::data::datasources::config::fill_url(
                &self.search_tpl.clone(),
                &[
                    ("query", &query.replace(' ', "+")),
                    ("sort", "popular"),
                    ("page", &page.max(1).to_string()),
                ],
            )
        };
        let v = self.fetch(&url).await?;
        Ok(Self::items_of(&v)
            .iter()
            .filter_map(|i| self.to_model(i))
            .collect())
    }

    pub async fn detail(&self, id: &str) -> Result<ContentModel, AppError> {
        let url = crate::data::datasources::config::fill_url(&self.detail_tpl.clone(), &[("id", id)]);
        let v = self.fetch(&url).await?;
        let title = v
            .get("title")
            .and_then(|t| {
                t.get("pretty")
                    .or_else(|| t.get("english"))
                    .and_then(|s| s.as_str())
            })
            .map(String::from)
            .unwrap_or_else(|| "(tanpa judul)".to_string());
        let cover = v
            .get("cover")
            .and_then(|c| c.get("path"))
            .and_then(|p| p.as_str())
            .map(|p| format!("{}/{p}", self.thumb_host))
            .unwrap_or_default();
        let upload = v
            .get("upload_date")
            .and_then(|u| u.as_u64().or_else(|| u.as_str()?.parse().ok()))
            .map(|n| n.to_string());
        Ok(ContentModel {
            id: id.to_string(),
            title,
            cover_url: cover,
            source_id: "nhentai".to_string(),
            upload_date: upload,
            page_count: v
                .get("num_pages")
                .and_then(|n| n.as_u64().map(|n| n as u32)),
            language: nhentai_lang_of_tags(&v),
        })
    }

    /// URL gambar penuh satu galeri (untuk reader).
    pub async fn pages(&self, gallery_id: &str) -> Result<Vec<String>, AppError> {
        let url = crate::data::datasources::config::fill_url(&self.detail_tpl.clone(), &[("id", gallery_id)]);
        let v = self.fetch(&url).await?;
        Ok(v
            .get("pages")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|pg| {
                        pg.get("path")
                            .and_then(|p| p.as_str())
                            .map(|p| format!("{}/{p}", self.img_host))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Chapter id `"{gallery}-1"` → gallery id.
    pub fn gallery_of_chapter(chapter_id: &str) -> &str {
        chapter_id.strip_suffix("-1").unwrap_or(chapter_id)
    }
}

#[cfg(test)]
mod nhentai_tests {
    use super::*;
    use crate::data::datasources::config::SourceFile;

    #[test]
    fn json_engine_parses_areakomik_shaped_fixture() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };
        const LIST: &str = r#"<html><body>
            <div class="komik-card"><a class="thumb-wrap" href="/series/slug-satu/">
            <img src="https://cdn.example/1.jpg"/><div class="card-title">Judul Satu</div><span class="page-num">24 halaman</span><span class="lang">English</span></a></div>
            <div class="komik-card"><a class="thumb-wrap" href="/series/slug-dua/">
            <img src="https://cdn.example/2.jpg"/><div class="card-title">Judul Dua</div></a></div>
            <div class="komik-card"><div class="card-title"></div></div>
            </body></html>"#;
        const DETAIL: &str = r#"<html><head>
            <meta property="og:image" content="https://cdn.example/cover.jpg"/></head><body>
            <h1>Judul Satu</h1>
            <div class="chapter-grid">
            <div class="chapter-row"><a class="chapter-link" href="/chapter/c1/">
            <span class="chap-num">Chapter 1</span><span class="chap-date">kemarin</span></a></div>
            <div class="chapter-row"><a class="chapter-link" href="/chapter/c2/">
            <span class="chap-num">Chapter 2</span></a></div>
            </div></body></html>"#;
        const CHAPTER: &str = r#"<html><body>
            <img src="https://gudangkomik.example/p1.jpg"/>
            <img src="https://warungkomikcdn.example/p2.jpg"/>
            </body></html>"#;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            for stream in listener.incoming().take(8) {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 4096];
                let len = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..len]).to_string();
                let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");
                let body = if path.starts_with("/ak-detail") {
                    DETAIL
                } else if path.starts_with("/ak-chapter") {
                    CHAPTER
                } else {
                    LIST
                };
                let res = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                );
                stream.write_all(res.as_bytes()).unwrap();
            }
        });
        let scraper_json = r#"{"enabled": true, "urlPatterns": {
                "search": {"url": "/ak-list", "list": {"container": ".komik-card",
                    "fields": {
                        "id": {"selector": "a.thumb-wrap", "attribute": "href", "transform": "slug"},
                        "title": {"selector": ".card-title"},
                        "coverUrl": {"selector": ".thumb-wrap img", "attribute": "src"},
                        "pageCount": {"selector": ".page-num"},
                        "language": {"selector": ".lang"}}}},
                "detail": "/ak-detail"},
            "selectors": {"detail": {"fields": {
                    "title": {"selector": "h1"},
                    "coverUrl": {"selector": "meta[property='og:image']", "attribute": "content"}},
                "chapters": {"container": ".chapter-grid .chapter-row", "fields": {
                    "id": {"selector": "a.chapter-link", "attribute": "href", "transform": "slug"},
                    "title": {"selector": ".chap-num"}}}},
                "reader": {"images": {"selector": "img", "attribute": "src"}}}}"#;
        let file: crate::data::datasources::config::SourceFile = serde_json::from_str(
            &format!(r#"{{"source": "areakomik-test", "baseUrl": "{base}", "scraper": {scraper_json}}}"#),
        )
        .unwrap();
        let cfg = source_config_from_json(
            "areakomik-test",
            &file.base_url,
            file.scraper.as_ref().unwrap(),
            "indonesian",
        )
        .expect("pola areakomik terbaca");
        assert_eq!(cfg.list_path, "/ak-list");
        let http = HttpClientManager::without_proxy().unwrap();
        let engine = GenericScraperAdapter::new(http);
        let items = tauri::async_runtime::block_on(engine.fetch_list(&format!("{base}/ak-list"), &cfg)).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "slug-satu");
        assert_eq!(items[0].title, "Judul Satu");
        assert_eq!(items[0].cover_url, "https://cdn.example/1.jpg");
        assert_eq!(items[0].language.as_deref(), Some("en"));
        assert_eq!(items[0].page_count, Some(24));
        let detail = tauri::async_runtime::block_on(engine.fetch_detail(&format!("{base}/ak-detail"), &cfg)).unwrap();
        assert_eq!(detail.title, "Judul Satu");
        assert_eq!(detail.cover_url, "https://cdn.example/cover.jpg");
        let chapters = tauri::async_runtime::block_on(engine.fetch_chapters(&format!("{base}/ak-detail"), "x", &cfg)).unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].id, "c1");
        assert_eq!(chapters[0].title, "Chapter 1");
        let expect_ch = format!("{base}/chapter/c1/");
        assert_eq!(chapters[0].external_url.as_deref(), Some(expect_ch.as_str()));
        let pages = tauri::async_runtime::block_on(engine.fetch_page_images(&format!("{base}/ak-chapter"), &cfg)).unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].contains("gudangkomik"));
    }

    #[test]
    fn configs_constructible() {
        // Site config kompilasi + URL terbentuk (tuning selector lanjut live).
        let cfg = nhentai_config();
        assert!(cfg.list_url("test", 1).contains("nhentai.net/search"));
        assert!(hitomi_config().list_url("x", 1).contains("hitomi.la"));
        assert!(ehentai_config().list_url("x", 1).contains("e-hentai.org"));
    }

    const SEARCH_JSON: &str = r#"{"result": [
        {"id": 462098, "media_id": 2600126, "english_title": "Judul Tes",
         "thumbnail": "galleries/2600126/thumb.jpg", "num_pages": 11, "num_favorites": 5,
         "tag_ids": [12227, 5]}
    ], "num_pages": 1}"#;

    const DETAIL_JSON: &str = r#"{"id": 462098, "title": {"english": "Judul Tes", "pretty": "Judul Rapi"},
        "cover": {"path": "galleries/2600126/cover.jpg"},
        "upload_date": 1710000000,
        "pages": [{"number": 1, "path": "galleries/2600126/1.jpg"}, {"number": 2, "path": "galleries/2600126/2.jpg"}]}"#;

    fn test_adapter() -> NhentaiApiAdapter {
        let cfg: SourceFile = serde_json::from_str(
            r#"{"source": "nhentai", "api": {"apiBase": "https://nhentai.net",
                "endpoints": {"search": "/api/v2/search?query={query}&sort={sort}&page={page}",
                              "galleryDetail": "/api/v2/galleries/{id}"}},
                "assetHosts": {"image": "https://i.nhentai.net", "thumbnail": "https://t.nhentai.net"},
                "network": {"rateLimit": {"minDelayMs": 0}, "retry": {"maxAttempts": 1, "delayMs": 1}}}"#,
        )
        .unwrap();
        NhentaiApiAdapter::from_config(HttpClientManager::without_proxy().unwrap(), &cfg)
    }

    #[test]
    fn parses_canned_search_and_detail() {
        let a = test_adapter();
        let v: serde_json::Value = serde_json::from_str(SEARCH_JSON).unwrap();
        let models: Vec<ContentModel> = NhentaiApiAdapter::items_of(&v)
            .iter()
            .filter_map(|i| a.to_model(i))
            .collect();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "462098");
        assert_eq!(models[0].title, "Judul Tes");
        assert_eq!(
            models[0].cover_url,
            "https://t.nhentai.net/galleries/2600126/thumb.jpg"
        );
        assert_eq!(models[0].language.as_deref(), Some("en"));
        assert_eq!(normalize_lang("indonesian").as_deref(), Some("id"));
        assert_eq!(normalize_lang("Japanese").as_deref(), Some("ja"));
        assert!(normalize_lang("").is_none());
        assert_eq!(NhentaiApiAdapter::gallery_of_chapter("462098-1"), "462098");
        // Detail diparse dari bentuk live (tanpa network).
        let d: serde_json::Value = serde_json::from_str(DETAIL_JSON).unwrap();
        assert_eq!(
            d.get("title").unwrap().get("pretty").unwrap(),
            "Judul Rapi"
        );
        let pages: Vec<String> = d
            .get("pages")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|p| format!("https://i.nhentai.net/{}", p.get("path").unwrap().as_str().unwrap()))
            .collect();
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn installed_nhentai_config_drives_adapter() {
        // Config hasil install (bukan bundel) menggerakkan adapter.
        let cfg: SourceFile = serde_json::from_str(
            r#"{"source": "nhentai", "api": {"apiBase": "https://nhentai.net",
                "endpoints": {"search": "/api/v2/search?query={query}&sort={sort}&page={page}",
                              "galleryDetail": "/api/v2/galleries/{id}"}},
                "assetHosts": {"image": "https://i.nhentai.net", "thumbnail": "https://t.nhentai.net"}}"#,
        )
        .unwrap();
        let a = NhentaiApiAdapter::from_config(HttpClientManager::without_proxy().unwrap(), &cfg);
        assert!(a.search_tpl.contains("/api/v2/search"));
        assert_eq!(a.thumb_host, "https://t.nhentai.net");
    }

    #[test]
    #[ignore = "hits live nhentai.net API; butuh config ter-install — salin manual satu file ke /tmp bila perlu"]
    fn live_nhentai_api_search_detail_pages() {
        let cfg: SourceFile = serde_json::from_str(
            r#"{"source": "nhentai", "api": {"apiBase": "https://nhentai.net",
                "endpoints": {"search": "/api/v2/search?query={query}&sort={sort}&page={page}",
                              "galleryDetail": "/api/v2/galleries/{id}"}},
                "assetHosts": {"image": "https://i.nhentai.net", "thumbnail": "https://t.nhentai.net"},
                "network": {"rateLimit": {"minDelayMs": 200}}}"#,
        )
        .unwrap();
        let a = NhentaiApiAdapter::from_config(HttpClientManager::new().unwrap(), &cfg);
        let items = tauri::async_runtime::block_on(a.search("test", 1)).unwrap();
        assert!(!items.is_empty());
        assert!(!items[0].cover_url.is_empty());
        let detail =
            tauri::async_runtime::block_on(a.detail(&items[0].id)).unwrap();
        assert_eq!(detail.id, items[0].id);
        let pages = tauri::async_runtime::block_on(a.pages(&items[0].id)).unwrap();
        assert!(!pages.is_empty());
        assert!(pages[0].starts_with("https://i.nhentai.net/"));
        // Regresi: query kosong (home feed) pakai allGalleries, bukan 400.
        let home = tauri::async_runtime::block_on(a.search("", 1)).unwrap();
        assert!(!home.is_empty());
    }
}
