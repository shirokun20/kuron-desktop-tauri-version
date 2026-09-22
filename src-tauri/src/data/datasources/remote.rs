//! Remote DataSources — port `kuron_generic` + `kuron_special` (Fase 2, spec §12).
//! Engine config-driven: `GenericScraperAdapter` (HTML + CSS selector) dan
//! `GenericRestAdapter` (JSON API). Adapter situs = config + path per situs.
//! Selector situs dituning manual (best-known); engine diuji via fixture lokal.

use std::collections::HashMap;

use scraper::{Html, Selector};

use crate::{
    core::AppError,
    data::{
        datasources::config::{encode_query, fill_url, SourceFile},
        models::ContentModel,
    },
    domain::{Chapter, Comment},
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
    /// Selector link halaman berikut (mis. `#unext`); token `?next=` mobile.
    /// Kosong = tanpa cursor (template `{page}` saja).
    pub pagination_next: String,
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
    /// Field detail config-driven (kosong = tak dipakai).
    pub detail_page_count: FieldMap,
    pub detail_language: FieldMap,
    /// Sinopsis (`detail.fields.description`, mobile `subTitle`).
    pub detail_description: FieldMap,
    /// Genre/tag multi (`detail.fields.tags|tag|genre|genres`).
    pub detail_genres: FieldMap,
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

/// Ekstrak SEMUA cocok (field `multi`, mis. genre) — kosong dibuang.
fn extract_all(scope: scraper::ElementRef<'_>, sel: &Selector, map: &FieldMap) -> Vec<String> {
    scope
        .select(sel)
        .map(|el| apply_map(raw_of(el, map), map))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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

/// Cover dari `style="...url(...)..."` (mobile `_extractCoverFromRow`:
/// e-hentai menaruh thumbnail di `.glthumb div`, bukan `<img>`).
fn style_cover_url(item: scraper::ElementRef<'_>) -> String {
    let Ok(sel) = Selector::parse("[style]") else {
        return String::new();
    };
    for el in item.select(&sel) {
        let style = el.value().attr("style").unwrap_or("");
        if let Some(u) = extract_style_url(style) {
            if !u.is_empty() && !u.starts_with("data:") {
                return u;
            }
        }
    }
    String::new()
}

fn extract_style_url(style: &str) -> Option<String> {
    let pos = style.to_lowercase().find("url(")?;
    let rest = style[pos + 4..].trim_start_matches(['"', '\'', ' ']);
    let end = rest.find(['"', '\'', ')']).unwrap_or(rest.len());
    Some(rest[..end].trim().replace("&amp;", "&"))
}

/// Bahasa dari tag baris `.gt[title="language:x"]`
/// (mobile `_extractLanguageFromRow`).
fn row_language(item: scraper::ElementRef<'_>) -> Option<String> {
    let Ok(sel) = Selector::parse(".gt[title]") else {
        return None;
    };
    for el in item.select(&sel) {
        let t = el.value().attr("title").unwrap_or("");
        if let Some((k, v)) = t.split_once(':') {
            if k.trim().eq_ignore_ascii_case("language") {
                let name = v.trim();
                if !name.is_empty() {
                    return Some(normalize_lang(name).unwrap_or_else(|| name.to_string()));
                }
            }
        }
    }
    None
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
            &[("query", query), ("q", query), ("page", &page.to_string())],
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
            // Pola `home` acap bawa `{page}` sendiri (ehentai `/?page={page}`):
            // isi bila ada, bukan URL mentah yang mengulang halaman 1.
            crate::data::datasources::config::fill_url(
                &self.home_path,
                &[("page", &page.to_string())],
            )
        } else {
            return self.list_url("", 1);
        };
        if path.starts_with("http://") || path.starts_with("https://") {
            return path;
        }
        format!("{}{}", self.base_url, path)
    }

    /// URL search dari payload `raw:k=v&..` (mobile `_searchRaw` disederhanakan):
    /// base path pola + default query template (raw menang) + substitusi
    /// `{page}`/`{query}`/`{tag}`. Kunci page dari raw dibuang (adapter/cursor
    /// yang kendalikan); sisa placeholder tak dikenal dibersihkan.
    pub fn search_url_raw(&self, raw: &str, page: u32) -> String {
        let payload = raw.strip_prefix("raw:").unwrap_or(raw);
        let raw_pairs = parse_raw_params(payload);
        let (mut base, tpl_query) = match self.list_path.split_once('?') {
            Some((b, q)) => (b.to_string(), q),
            None => (self.list_path.clone(), ""),
        };
        base = base.replace("{page}", &page.to_string());
        let query_val = raw_query_value(&raw_pairs, self.query_key_hint());
        let tag_val = raw_first(&raw_pairs, &["tag"])
            .map(|t| t.to_lowercase().replace(' ', "-"))
            .unwrap_or_default();
        base = base
            .replace(
                "{query}",
                &crate::data::datasources::config::encode_query(&query_val),
            )
            .replace(
                "{tag}",
                &crate::data::datasources::config::encode_query(&tag_val),
            );
        base = clear_placeholders(&base);
        // Pasangan default template.
        let mut merged: Vec<(String, String)> = Vec::new();
        let mut consumed: Vec<String> =
            vec!["page".to_string(), "paged".to_string(), "p".to_string()];
        for pair in tpl_query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = match pair.split_once('=') {
                Some(x) => x,
                None => (pair, ""),
            };
            let filled = if v.contains("{page}") {
                v.replace("{page}", &page.to_string())
            } else if v.contains("{query}") {
                consumed.push(k.to_string());
                consumed.push("query".to_string());
                consumed.push("q".to_string());
                v.replace(
                    "{query}",
                    &crate::data::datasources::config::encode_query(&query_val),
                )
            } else if v.contains("{tag}") {
                consumed.push(k.to_string());
                consumed.push("tag".to_string());
                v.replace(
                    "{tag}",
                    &crate::data::datasources::config::encode_query(&tag_val),
                )
            } else {
                v.to_string()
            };
            merged.push((k.to_string(), filled));
        }
        // Raw menang atas default; nilai kosong tak menimpa default berisi.
        for (k, v) in raw_pairs {
            if consumed.iter().any(|c| c == &k) || v.is_empty() {
                continue;
            }
            if let Some(slot) = merged.iter_mut().find(|(ek, _)| ek == &k) {
                slot.1 = crate::data::datasources::config::encode_query(&v);
            } else {
                merged.push((k, crate::data::datasources::config::encode_query(&v)));
            }
        }
        // Bersihkan default yang nilainya kosong (mis. `q=` sisa).
        let merged: Vec<(String, String)> = merged
            .into_iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .collect();
        let url = if merged.is_empty() {
            base
        } else {
            format!(
                "{base}?{}",
                merged
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        };
        if url.starts_with("http://") || url.starts_with("https://") {
            return url;
        }
        format!("{}{}", self.base_url, url)
    }

    /// Nama kunci query template (`f_search={query}` → `f_search`) untuk
    /// prioritas nilai query-ish dari raw.
    fn query_key_hint(&self) -> &str {
        for pair in self.list_path.split('?').nth(1).unwrap_or("").split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if v.contains("{query}") || v.contains("{q}") {
                    return k;
                }
            }
        }
        "query"
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
        absolutize_url(&self.base_url, src)
    }
}

/// Parse `k=v&k2=v2` ala mobile `_parseRawQueryParams` (decode dulu).
fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter_map(|pair| {
            if pair.is_empty() {
                return None;
            }
            let (k, v) = pair.split_once('=')?;
            Some((decode_query_component(k), decode_query_component(v)))
        })
        .collect()
}

/// Decode `+`/​`%XX` ala `Uri.decodeQueryComponent`; sekuen rusak dibiarkan.
fn decode_query_component(s: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'+' {
            bytes.push(b' ');
            i += 1;
        } else if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex_val(b[i + 1]), hex_val(b[i + 2])) {
                bytes.push(h * 16 + l);
                i += 3;
            } else {
                bytes.push(b[i]);
                i += 1;
            }
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Nilai non-kosong pertama dari daftar kunci (berurutan).
fn raw_first(pairs: &[(String, String)], keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some((_, v)) = pairs
            .iter()
            .find(|(ek, ev)| ek == *k && !ev.trim().is_empty())
        {
            return Some(v.clone());
        }
    }
    None
}

/// Nilai query teks: kunci template dulu, lalu kandidat umum.
fn raw_query_value(pairs: &[(String, String)], hint: &str) -> String {
    let mut keys = vec![
        hint, "query", "q", "s", "keyword", "search", "title", "f_search",
    ];
    keys.dedup();
    raw_first(pairs, &keys).unwrap_or_default()
}

/// Buang sisa `{placeholder}` agar tak bocor sebagai `%7B..%7D`.
fn clear_placeholders(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(a) = out.find('{') {
        if let Some(b) = out[a..].find('}') {
            out.replace_range(a..a + b + 1, "");
        } else {
            break;
        }
    }
    out
}

/// Kunci query mentah ada (pencocokan nama persis sebelum `=`, URL-decoded).
fn has_query_key(url: &str, key: &str) -> bool {
    let Some(q) = url.split_once('?').map(|(_, q)| q) else {
        return false;
    };
    q.split('&')
        .any(|pair| pair.split_once('=').map(|(k, _)| k).unwrap_or(pair) == key)
}

/// Ganti total param multi-nilai (mobile `_replaceMultiValueParam`):
/// semua kemunculan `key=` dibuang, lalu nilai baru ditempel berurutan.
fn replace_multi_param(url: &str, key: &str, values: &[String]) -> String {
    let (base, query) = match url.split_once('?') {
        Some((b, q)) => (b.to_string(), q.to_string()),
        None => (url.to_string(), String::new()),
    };
    let mut kept: Vec<String> = Vec::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let name = pair.split_once('=').map(|(k, _)| k).unwrap_or(pair);
        if name != key {
            kept.push(pair.to_string());
        }
    }
    for v in values {
        kept.push(format!("{key}={}", encode_query(v)));
    }
    if kept.is_empty() {
        base
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}
fn absolutize_url(base: &str, src: &str) -> String {
    if src.is_empty() {
        return String::new();
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        src.to_string()
    } else if let Some(rest) = src.strip_prefix("//") {
        format!("https://{rest}")
    } else if src.starts_with('/') {
        format!("{base}{src}")
    } else {
        format!("{base}/{src}")
    }
}

/// Cache cursor pagination token ala mobile (`_paginationCursorCache` +
/// `_pageUrlCache`): halaman N+1 diambil dari link `next` halaman N,
/// bukan template `?page=` (E-Hentai mengabaikan `?page=` — isi hal 1 lagi).
#[derive(Debug, Clone, Default)]
pub struct PaginationCursors {
    inner: std::sync::Arc<std::sync::Mutex<HashMap<String, CursorState>>>,
}

#[derive(Debug, Clone)]
pub enum CursorState {
    Next(String),
    Exhausted,
}

impl PaginationCursors {
    /// Kunci cursor: `home:{source}:{page}` / `search:{source}:{query}:{page}`.
    pub fn home_key(source: &str, page: u32) -> String {
        format!("home:{source}:{page}")
    }

    pub fn search_key(source: &str, query: &str, page: u32) -> String {
        format!("search:{source}:{}:{page}", query.trim())
    }

    pub fn get(&self, key: &str) -> Option<CursorState> {
        self.inner.lock().ok()?.get(key).cloned()
    }

    pub fn put_next(&self, next_page_key: &str, url: String) {
        if let Ok(mut m) = self.inner.lock() {
            m.insert(next_page_key.to_string(), CursorState::Next(url));
        }
    }

    pub fn mark_exhausted(&self, next_page_key: &str) {
        if let Ok(mut m) = self.inner.lock() {
            m.insert(next_page_key.to_string(), CursorState::Exhausted);
        }
    }
}

/// Link halaman berikut ala mobile (`_extractSearchNavUrl`): selector config
/// (`#unext`), cadangan `#dnext` / `.searchnav a[href*=next=]`, lalu
/// `var nexturl="..."` di script.
fn extract_next_url(html: &str, base_url: &str, selector: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let mut selectors: Vec<&str> = Vec::new();
    if !selector.trim().is_empty() {
        selectors.push(selector.trim());
    }
    selectors.extend(["#dnext", ".searchnav a[href*=\"next=\"]"]);
    for sel_str in selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            if let Some(el) = doc.select(&sel).next() {
                let href = el
                    .value()
                    .attr("href")
                    .unwrap_or("")
                    .trim()
                    .replace("&amp;", "&");
                if !href.is_empty() {
                    return Some(absolutize_url(base_url, &href));
                }
            }
        }
    }
    // Fallback script: `var nexturl="..."`.
    let lower = html.to_lowercase();
    if let Some(pos) = lower.find("var nexturl") {
        let rest = &html[pos..];
        if let Some(q1) = rest.find('"') {
            let after = &rest[q1 + 1..];
            if let Some(q2) = after.find('"') {
                let url = after[..q2].trim().replace("&amp;", "&").replace(r"\/", "/");
                if !url.is_empty() {
                    return Some(absolutize_url(base_url, &url));
                }
            }
        }
    }
    None
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
        pagination_next: String::new(),
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
        detail_page_count: FieldMap::default(),
        detail_description: FieldMap::default(),
        detail_genres: FieldMap::default(),
        detail_language: FieldMap::default(),
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
        pagination_next: String::new(),
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
        detail_page_count: FieldMap::default(),
        detail_description: FieldMap::default(),
        detail_genres: FieldMap::default(),
        detail_language: FieldMap::default(),
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

/// E-Hentai — salinan nilai `informations/configs/ehentai-config.json` mobile.
/// Fallback bila JSON installed tak terbaca; `repo_for` utamakan JSON.
pub fn ehentai_config() -> SourceConfig {
    SourceConfig {
        source_id: "ehentai".to_string(),
        base_url: "https://e-hentai.org".to_string(),
        list_path: "/?f_search={query}&page={page}".to_string(),
        home_path: "/?page={page}".to_string(),
        home_page_path: String::new(),
        pagination_next: "#unext".to_string(),
        // ID = regex config: `/g/([0-9]+/[a-zA-Z0-9]+)/`.
        detail_path: "/g/{id}/".to_string(),
        item_selector: ".itg.gltc tr".to_string(),
        id: FieldMap {
            selector: ".gl3c.glname a".to_string(),
            attribute: Some("href".to_string()),
            regex: Some("/g/([0-9]+/[a-zA-Z0-9]+)/".to_string()),
            ..Default::default()
        },
        title: FieldMap::text(".glink"),
        link: FieldMap::attr(".gl3c.glname a", "href"),
        cover: FieldMap::attr(".gl2c img", "src"),
        detail_title: FieldMap::text("#gn"),
        detail_cover: FieldMap::attr("#gd1 img", "src"),
        detail_page_count: FieldMap {
            selector: "#gdd tr:nth-child(6) td.gdt2".to_string(),
            regex: Some("([0-9]+)".to_string()),
            ..Default::default()
        },
        detail_description: FieldMap::default(),
        detail_genres: FieldMap::default(),
        detail_language: FieldMap::default(),
        // Single-gallery (mobile bangun Part chapters dari paginasi ?p=).
        chapter_selector: String::new(),
        chapter_link: FieldMap::default(),
        chapter_title: FieldMap::default(),
        page_selector: "#img".to_string(),
        page_attr: Some("src".to_string()),
        page_count: FieldMap::default(),
        language: FieldMap::default(),
        default_language: Some("unknown".to_string()),
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
    let (chapter_selector, chapter_link, chapter_title) = chapters_from(&scraper.selectors);
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
    let detail_page_count = dfield(&["pageCount"]);
    let detail_page_count = if detail_page_count.selector.is_empty() {
        FieldMap::default()
    } else {
        detail_page_count
    };
    let detail_language = dfield(&["language"]);
    let detail_language = if detail_language.selector.is_empty() {
        FieldMap::default()
    } else {
        detail_language
    };
    // Mobile `_extractDescription` (subTitle→description) + `_resolveTags`
    // (tags→tag→genre→genres): kunci pertama yang ADA selector-nya.
    let detail_description = dfield(&["subTitle", "description"]);
    let detail_description = if detail_description.selector.is_empty() {
        FieldMap::default()
    } else {
        detail_description
    };
    let detail_genres = dfield(&["tags", "tag", "genre", "genres"]);
    let detail_genres = if detail_genres.selector.is_empty() {
        FieldMap::default()
    } else {
        detail_genres
    };
    // Cursor token mobile (`list.pagination.next`, mis. `#unext`).
    let pagination_next = list.pagination.get("next").cloned().unwrap_or_default();
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
        detail_page_count,
        detail_language,
        detail_description,
        detail_genres,
        page_count,
        language,
        pagination_next,
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

/// Fallback `languageTagMap` = salinan nilai config bundel (pola endpoint).
fn default_language_tag_map() -> std::collections::HashMap<String, String> {
    [
        ("6346".to_string(), "japanese".to_string()),
        ("12227".to_string(), "english".to_string()),
        ("29963".to_string(), "chinese".to_string()),
    ]
    .into_iter()
    .collect()
}

/// Bahasa NHentai dari tags detail (mobile `firstWhere`: type=language,
/// nama tak MENGANDUNG "translated").
/// Parse `tags[]` detail nhentai → `Tag` (murni).
/// Bentuk live: `{id, type, name, slug, url, count}` — toleran: entri
/// tanpa nama dilewati, angka tak valid jadi 0 (ala fallback mobile).
fn parse_nhentai_tags(v: &serde_json::Value) -> Vec<crate::domain::Tag> {
    v.get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(crate::domain::Tag {
                        id: t
                            .get("id")
                            .and_then(|n| {
                                n.as_i64()
                                    .map(|n| n.to_string())
                                    .or_else(|| n.as_str().map(str::to_string))
                            })
                            .unwrap_or_else(|| "0".to_string()),
                        name: t.get("name")?.as_str()?.to_string(),
                        tag_type: t
                            .get("type")
                            .and_then(|x| x.as_str())
                            .unwrap_or("tag")
                            .to_string(),
                        count: t.get("count").and_then(|n| n.as_i64()).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn nhentai_lang_of_tags(v: &serde_json::Value) -> Option<String> {
    let tags = v.get("tags")?.as_array()?;
    for t in tags {
        if t.get("type").and_then(|x| x.as_str()) != Some("language") {
            continue;
        }
        let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name.to_lowercase().contains("translated") {
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
/// Cover builder config: `https://mangadex.org/covers/{mangaId}/{fileName}.512.jpg`.
const MANGADEX_COVER_TPL: &str = "https://mangadex.org/covers/{id}/{file}.512.jpg";
const MANGADEX_PAGE_SIZE: u32 = 100;
const MANGADEX_MAX_OFFSET: u32 = 9900;
const MANGADEX_ALL: &str = "/manga?limit=100&offset={offset}&includes[]=cover_art&includes[]=author&includes[]=artist&contentRating[]=erotica&contentRating[]=pornographic&contentRating[]=suggestive&contentRating[]=safe&hasAvailableChapters=true&order[latestUploadedChapter]=desc";
const MANGADEX_SEARCH: &str = "/manga?title={query}&limit=100&offset={offset}&includes[]=cover_art&includes[]=author&includes[]=artist&contentRating[]=erotica&contentRating[]=pornographic&contentRating[]=suggestive&contentRating[]=safe&hasAvailableChapters=true";
const MANGADEX_DETAIL: &str =
    "/manga/{id}?includes[]=cover_art&includes[]=author&includes[]=artist";
const MANGADEX_CHAPTERS: &str = "/chapter?manga={id}&translatedLanguage[]={language}&limit=100&order[chapter]=desc&contentRating[]=safe&contentRating[]=suggestive&contentRating[]=erotica&contentRating[]=pornographic";
const MANGADEX_AT_HOME: &str = "/at-home/server/{chapterId}";
/// Statistik (mobile `api.statistics.followsEndpoint`): follows + rating.
const MANGADEX_STATS: &str = "/statistics/manga/{id}";
/// Terkait fase-1 (mobile `api.related.batch.idEndpoint`): rekomendasi.
const MANGADEX_RECOMMEND: &str = "/manga/{id}/recommendation?includes[]=manga";
/// Terkait fase-2 (mobile `itemsEndpoint`): batch `ids[]`, parse = list.
const MANGADEX_BY_IDS: &str =
    "/manga?ids[]={ids}&includes[]=cover_art&includes[]=author&includes[]=artist&limit=100";

/// Bahasa detail dari `<div id="td_language:x">` (mobile `_extractTags`).
fn detail_language_tag(doc: &Html) -> Option<String> {
    let Ok(sel) = Selector::parse("[id^=\"td_language\"]") else {
        return None;
    };
    for el in doc.select(&sel) {
        let id = el.value().attr("id").unwrap_or("");
        let name = id
            .split_once(':')
            .map(|(_, v)| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .or_else(|| {
                let t = collapse_ws(&el.text().collect::<String>());
                (!t.is_empty()).then_some(t)
            });
        if let Some(name) = name {
            if name.eq_ignore_ascii_case("translated") {
                continue;
            }
            return Some(normalize_lang(&name).unwrap_or(name));
        }
    }
    None
}

pub struct GenericRestAdapter {
    http: HttpClientManager,
    /// JSON `mangadex-config.json` installed; endpoint dibaca dari sini.
    source_file: Option<SourceFile>,
}

impl GenericRestAdapter {
    pub fn new(http: HttpClientManager) -> Self {
        Self {
            http,
            source_file: None,
        }
    }

    pub fn with_source_file(mut self, file: SourceFile) -> Self {
        self.source_file = Some(file);
        self
    }

    fn md_base(&self) -> String {
        self.source_file
            .as_ref()
            .map(|f| f.base_url.clone())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| MANGADEX_API.to_string())
    }

    /// Endpoint `api.endpoints[name]` dari JSON, fallback template bawaan
    /// (salinan nilai `mangadex-config.json` mobile).
    fn md_endpoint(&self, name: &str, params: &[(&str, &str)], fallback: &str) -> String {
        if let Some(url) = self
            .source_file
            .as_ref()
            .and_then(|f| f.endpoint(name, params))
        {
            return url;
        }
        let path = fill_url(fallback, params);
        format!("{}{}", self.md_base(), path)
    }

    /// Cari manga; query kosong → endpoint `allGalleries` (mobile `_searchNewSchema`).
    /// Prefix `raw:` → param mentah digabung ke template (mobile `_buildRawSearchUrl`).
    pub async fn search_mangadex(
        &self,
        query: &str,
        page: u32,
    ) -> Result<Vec<ContentModel>, AppError> {
        let page = page.max(1);
        let offset =
            ((page.saturating_sub(1) * MANGADEX_PAGE_SIZE).min(MANGADEX_MAX_OFFSET)).to_string();
        let url = if query.trim().is_empty() {
            self.md_endpoint("allGalleries", &[("offset", &offset)], MANGADEX_ALL)
        } else if let Some(raw) = query.strip_prefix("raw:") {
            self.md_endpoint_raw("search", raw, &offset, MANGADEX_SEARCH)
        } else {
            self.md_endpoint(
                "search",
                &[("query", query), ("offset", &offset)],
                MANGADEX_SEARCH,
            )
        };
        let body = self.http.get(&url, "mangadex").await?;
        Self::parse_mangadex(&body)
    }

    /// Endpoint + merge param `raw:` (mobile `_buildRawSearchUrl` +
    /// `_applyQueryRules`): placeholder `{k}` diisi nilai raw pertama
    /// (`{offset}` dari paging), pasangan template berisi dipertahankan
    /// (raw menimpa semua kemunculan kunci), `rawParam` = fragmen mentah.
    fn md_endpoint_raw(&self, name: &str, raw: &str, offset: &str, fallback: &str) -> String {
        let template = self
            .source_file
            .as_ref()
            .and_then(|f| {
                f.api
                    .as_ref()?
                    .endpoints
                    .get(name)?
                    .as_str()
                    .map(str::to_string)
            })
            .unwrap_or_else(|| fallback.to_string());
        let (base, tpl_query) = match template.split_once('?') {
            Some((b, q)) => (b.to_string(), q),
            None => (template.clone(), ""),
        };
        let raw_pairs = parse_raw_params(raw);
        let first = |k: &str| {
            raw_pairs
                .iter()
                .find(|(ek, ev)| ek == k && !ev.trim().is_empty())
                .map(|(_, v)| v.clone())
        };
        let mut raw_fragments: Vec<String> = Vec::new();
        let mut merged: Vec<(String, String)> = Vec::new();
        let mut consumed: Vec<String> = Vec::new();
        for pair in tpl_query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = match pair.split_once('=') {
                Some(x) => x,
                None => (pair, ""),
            };
            // Substitusi `{ph}` satu per satu; literal template utuh.
            let mut out_val = String::new();
            let mut rest = v;
            while let Some(a) = rest.find('{') {
                out_val.push_str(&rest[..a]);
                let seg = &rest[a..];
                match seg.find('}') {
                    Some(b) => {
                        let ph = seg[1..b].to_string();
                        let val = if ph == "offset" {
                            offset.to_string()
                        } else {
                            first(&ph).unwrap_or_default()
                        };
                        if !val.is_empty() {
                            consumed.push(ph);
                        }
                        out_val.push_str(&encode_query(&val));
                        rest = &seg[b + 1..];
                    }
                    None => {
                        out_val.push_str(seg);
                        rest = "";
                    }
                }
            }
            out_val.push_str(rest);
            // Pasangan kosong (placeholder tanpa nilai) dibuang ala mobile.
            if !out_val.trim().is_empty() {
                merged.push((k.to_string(), out_val));
            }
        }
        // Raw menang ala mobile (`merged[key] = SEMUA nilai`): kunci raw
        // menggusur kemunculan TEMPLATE, tapi nilai raw sendiri menumpuk
        // utuh (multi-value: 4 excludedTags[] tetap 4, bukan 1 terakhir).
        let mut raw_keys: Vec<String> = Vec::new();
        let mut raw_out: Vec<(String, String)> = Vec::new();
        for (k, v) in raw_pairs {
            if k == "offset" || v.trim().is_empty() || consumed.iter().any(|c| c == &k) {
                continue;
            }
            if k == "rawParam" {
                // Fragmen `order[x]=y` ditempel utuh (tanpa bungkus `rawParam=`).
                raw_fragments.push(v);
                continue;
            }
            if !raw_keys.contains(&k) {
                raw_keys.push(k.clone());
            }
            raw_out.push((k, encode_query(&v)));
        }
        merged.retain(|(ek, _)| !raw_keys.contains(ek));
        merged.extend(raw_out);
        let mut url = if merged.is_empty() {
            base
        } else {
            format!(
                "{base}?{}",
                merged
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        };
        if !raw_fragments.is_empty() {
            let sep = if url.contains('?') { "&" } else { "?" };
            url = format!("{url}{sep}{}", raw_fragments.join("&"));
        }
        if url.starts_with("http://") || url.starts_with("https://") {
            return url;
        }
        let url = format!("{}{}", self.md_base(), url);
        Self::apply_query_rules(&url, self.md_query_rules("search"))
    }

    /// Aturan `api.queryRules[scope]` dari JSON (mobile `_applyQueryRules`).
    fn md_query_rules(&self, scope: &str) -> Option<serde_json::Value> {
        self.source_file
            .as_ref()
            .and_then(|f| f.api.as_ref()?.query_rules.get(scope).cloned())
    }

    /// `ensureParams` (tambah bila hilang), `enforceMultiValueParams`
    /// (ganti total), `ensureMultiValueParamsIfMissing` (tambah bila hilang).
    fn apply_query_rules(url: &str, rules: Option<serde_json::Value>) -> String {
        let Some(rules) = rules else {
            return url.to_string();
        };
        let Some(obj) = rules.as_object() else {
            return url.to_string();
        };
        let str_list = |v: &serde_json::Value| -> Vec<String> {
            v.as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|e| e.as_str().map(str::to_string))
                        .filter(|s| !s.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut out = url.to_string();
        if let Some(enforce) = obj
            .get("enforceMultiValueParams")
            .and_then(|v| v.as_object())
        {
            for (key, vals) in enforce {
                let values = str_list(vals);
                if values.is_empty() {
                    continue;
                }
                out = replace_multi_param(&out, key, &values);
            }
        }
        if let Some(ensure) = obj.get("ensureParams").and_then(|v| v.as_object()) {
            for (key, val) in ensure {
                let s = val.as_str().unwrap_or(&val.to_string()).trim().to_string();
                if s.is_empty() || has_query_key(&out, key) {
                    continue;
                }
                let sep = if out.contains('?') { "&" } else { "?" };
                out = format!("{out}{sep}{key}={}", encode_query(&s));
            }
        }
        if let Some(ensure_multi) = obj
            .get("ensureMultiValueParamsIfMissing")
            .and_then(|v| v.as_object())
        {
            for (key, vals) in ensure_multi {
                let values = str_list(vals);
                if values.is_empty() || has_query_key(&out, key) {
                    continue;
                }
                for v in values {
                    let sep = if out.contains('?') { "&" } else { "?" };
                    out = format!("{out}{sep}{key}={}", encode_query(&v));
                }
            }
        }
        out
    }

    /// Detail satu manga via API (`/manga/{id}`) + enrichment mobile:
    /// `availableTranslatedLanguages`, `description`, `tags[]`, lalu
    /// statistik (`api.statistics`, fail-soft ala mobile: gagal → None).
    pub async fn get_mangadex_detail(&self, id: &str) -> Result<ContentModel, AppError> {
        let url = self.md_endpoint("detail", &[("id", id)], MANGADEX_DETAIL);
        let body = self.http.get(&url, "mangadex").await?;
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| AppError::Network(format!("mangadex json: {e}")))?;
        let empty = vec![];
        let data = v.get("data").map(|d| vec![d.clone()]).unwrap_or(empty);
        let json = serde_json::json!({ "data": data });
        let mut items = Self::parse_mangadex(&json.to_string())?;
        let mut model = items
            .pop()
            .ok_or_else(|| AppError::Validation(format!("mangadex: {id} tidak ditemukan")))?;
        if let Some(attrs) = v.get("data").and_then(|d| d.get("attributes")) {
            model.tags = Self::md_tags_of(attrs);
            model.available_languages = Self::md_avail_langs_of(attrs);
            model.description = Self::md_description_of(attrs);
        }
        // Statistik fail-soft: detail tetap utuh bila endpoint gagal.
        let stats_url = self.md_endpoint("statistics", &[("id", id)], MANGADEX_STATS);
        if let Ok(stats_body) = self.http.get(&stats_url, "mangadex").await {
            let (follows, rating) = Self::parse_md_stats(&stats_body, id);
            model.favorites = follows;
            model.rating = rating;
        }
        Ok(model)
    }

    /// Tag MD (mobile `fields.tags`): id UUID, nama `name.en`,
    /// tipe = `group` (genre/theme/format/content); count tak ada → 0.
    fn md_tags_of(attrs: &serde_json::Value) -> Vec<crate::domain::Tag> {
        let empty = vec![];
        let arr = attrs
            .get("tags")
            .and_then(|t| t.as_array())
            .unwrap_or(&empty);
        arr.iter()
            .filter_map(|t| {
                let tag_attrs = t.get("attributes")?;
                let names = tag_attrs.get("name")?;
                let name = names
                    .get("en")
                    .and_then(|s| s.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        names
                            .as_object()?
                            .values()
                            .filter_map(|s| s.as_str())
                            .find(|s| !s.is_empty())
                    })?;
                Some(crate::domain::Tag {
                    id: t
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string(),
                    name: name.to_string(),
                    tag_type: tag_attrs
                        .get("group")
                        .and_then(|g| g.as_str())
                        .unwrap_or("tag")
                        .to_string(),
                    count: 0,
                })
            })
            .collect()
    }

    /// Bahasa tersedia (mobile `fields.availableTranslatedLanguages`).
    fn md_avail_langs_of(attrs: &serde_json::Value) -> Vec<String> {
        attrs
            .get("availableTranslatedLanguages")
            .and_then(|l| l.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l.as_str())
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Sinopsis (mobile `fields.description`): `en` lalu bahasa lain.
    fn md_description_of(attrs: &serde_json::Value) -> Option<String> {
        let desc = attrs.get("description")?;
        if let Some(en) = desc
            .get("en")
            .and_then(|s| s.as_str())
            .filter(|s| !s.is_empty())
        {
            return Some(en.to_string());
        }
        desc.as_object()?
            .values()
            .filter_map(|s| s.as_str())
            .find(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// Statistik (mobile `followsPath`/`ratingPath`): `(follows, rating)`.
    fn parse_md_stats(body: &str, id: &str) -> (Option<i64>, Option<f64>) {
        let stats = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| Some(v.get("statistics")?.get(id)?.clone()));
        let follows = stats
            .as_ref()
            .and_then(|s| s.get("follows"))
            .and_then(|f| f.as_i64().or_else(|| f.as_u64().map(|n| n as i64)));
        let rating = stats
            .as_ref()
            .and_then(|s| s.get("rating"))
            .and_then(|r| r.get("average"))
            .and_then(|a| a.as_f64());
        (follows, rating)
    }

    /// Terkait MD batch 2 fase (mobile `_fetchRelatedBatch`):
    /// rekomendasi → id (buang self, dedup, urut skor) → batch `ids[]`
    /// parse = list fields, diurut balik ke urutan fase-1.
    pub async fn related_mangadex(&self, manga_id: &str) -> Result<Vec<ContentModel>, AppError> {
        let url = self.md_endpoint("recommendation", &[("id", manga_id)], MANGADEX_RECOMMEND);
        let body = self.http.get(&url, "mangadex").await?;
        let ids = Self::parse_md_recommend_ids(&body, manga_id);
        if ids.is_empty() {
            return Ok(vec![]);
        }
        // `{ids}` → `ids[]=a&ids[]=b…` TANPA encode (`&` pemisah sah).
        let template = self
            .source_file
            .as_ref()
            .and_then(|f| f.api.as_ref())
            .and_then(|a| a.endpoints.get("mangaByIds"))
            .and_then(|e| e.as_str())
            .unwrap_or(MANGADEX_BY_IDS);
        let url = format!(
            "{}{}",
            self.md_base(),
            template.replace("{ids}", &ids.join("&ids[]="))
        );
        let body = self.http.get(&url, "mangadex").await?;
        let mut items = Self::parse_mangadex(&body)?;
        items.sort_by_key(|m| ids.iter().position(|id| id == &m.id).unwrap_or(usize::MAX));
        Ok(items)
    }

    /// Id rekomendasi (mobile `idItems`+`idField`): `data[*]` →
    /// `relationships[type==manga].id`, buang self + dedup (murni).
    fn parse_md_recommend_ids(body: &str, self_id: &str) -> Vec<String> {
        let empty = vec![];
        let v: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        let data = v.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for item in data {
            let rels = item
                .get("relationships")
                .and_then(|r| r.as_array())
                .cloned()
                .unwrap_or_default();
            for r in rels {
                if r.get("type").and_then(|t| t.as_str()) != Some("manga") {
                    continue;
                }
                if let Some(id) = r.get("id").and_then(|i| i.as_str()) {
                    if !id.is_empty() && id != self_id && seen.insert(id.to_string()) {
                        out.push(id.to_string());
                    }
                }
            }
        }
        out
    }

    /// URL chapter feed (murni, mobile `fetchChapters`): isi `{id}` +
    /// `{language}` (kosong → param dibuang), lalu UPSERT `offset`
    /// (load-more bottomsheet; contoh mobile `loadedForLang`).
    fn md_chapters_url(
        template: &str,
        base: &str,
        manga_id: &str,
        lang: Option<&str>,
        offset: Option<u32>,
    ) -> String {
        let filled = fill_url(
            template,
            &[("id", manga_id), ("language", lang.unwrap_or(""))],
        );
        // `{language}` kosong tak ter-strip otomatis (nama param beda) →
        // buang manual, HANYA saat tanpa filter bahasa.
        let cleaned = if lang.is_none() {
            filled
                .replace("&translatedLanguage[]=", "&")
                .replace("?translatedLanguage[]=&", "?")
                .replace("?translatedLanguage[]=", "?")
        } else {
            filled
        };
        let mut url = format!("{base}{cleaned}");
        if let Some(o) = offset.filter(|o| *o > 0) {
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&format!("offset={o}"));
        }
        url
    }

    /// Chapter feed (`api.detail.chapters`, mobile `fetchChapters` +
    /// `loadChapterLane`): bahasa eksplisit (chip UI) → satu fetch;
    /// None → coba `en` dulu, fallback tanpa filter bahasa.
    /// `offset` = load-more (None/0 = halaman pertama).
    pub async fn chapters_mangadex(
        &self,
        manga_id: &str,
        language: Option<&str>,
        offset: Option<u32>,
    ) -> Result<Vec<Chapter>, AppError> {
        let template: String = self
            .source_file
            .as_ref()
            .and_then(|f| f.api.as_ref())
            .and_then(|a| a.detail.get("chapters"))
            .and_then(|c| c.get("endpoint"))
            .and_then(|e| e.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| MANGADEX_CHAPTERS.to_string());
        let explicit = language.map(str::trim).filter(|l| !l.is_empty());
        let tries: Vec<Option<&str>> = match explicit {
            Some(l) => vec![Some(l)],
            None => vec![Some("en"), None],
        };
        for lang in tries {
            let url = Self::md_chapters_url(&template, &self.md_base(), manga_id, lang, offset);
            let body = self.http.get(&url, "mangadex").await?;
            let items = Self::parse_mangadex_chapters(&body, manga_id, offset.unwrap_or(0))?;
            if !items.is_empty() || lang.is_none() {
                return Ok(items);
            }
        }
        Ok(vec![])
    }

    /// Gambar halaman via at-home (`api.images.atHomeEndpoint`, 1 request).
    pub async fn pages_mangadex_at_home(&self, chapter_id: &str) -> Result<Vec<String>, AppError> {
        let template: String = self
            .source_file
            .as_ref()
            .and_then(|f| f.api.as_ref())
            .and_then(|a| a.images.get("atHomeEndpoint"))
            .and_then(|e| e.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| MANGADEX_AT_HOME.to_string());
        let url = format!(
            "{}{}",
            self.md_base(),
            fill_url(&template, &[("chapterId", chapter_id), ("id", chapter_id)])
        );
        let body = self.http.get(&url, "mangadex").await?;
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| AppError::Network(format!("mangadex at-home json: {e}")))?;
        let base = v.get("baseUrl").and_then(|b| b.as_str()).unwrap_or("");
        let node = v.get("chapter");
        let hash = node
            .and_then(|c| c.get("hash"))
            .and_then(|h| h.as_str())
            .unwrap_or("");
        let files = node
            .and_then(|c| c.get("data"))
            .and_then(|d| d.as_array())
            .filter(|a| !a.is_empty())
            .map(|a| (a, "data"))
            .or_else(|| {
                node.and_then(|c| c.get("dataSaver"))
                    .and_then(|d| d.as_array())
                    .map(|a| (a, "data-saver"))
            });
        let Some((arr, seg)) = files else {
            return Err(AppError::Network(
                "mangadex at-home: payload kosong".to_string(),
            ));
        };
        if base.is_empty() || hash.is_empty() {
            return Err(AppError::Network(
                "mangadex at-home: base/hash hilang".to_string(),
            ));
        }
        Ok(arr
            .iter()
            .filter_map(|f| f.as_str())
            .filter(|f| !f.is_empty())
            .map(|f| format!("{base}/{seg}/{hash}/{f}"))
            .collect())
    }

    /// Urutan = posisi feed API (+`base_offset` load-more). Mobile TANPA
    /// sorting (entity Chapter tanpa order); parse nomor chapter dilarang
    /// (desimal/sufiks/oneshot gagal → skema campur → urutan ngaco).
    fn parse_mangadex_chapters(
        body: &str,
        manga_id: &str,
        base_offset: u32,
    ) -> Result<Vec<Chapter>, AppError> {
        let v: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| AppError::Network(format!("mangadex chapters json: {e}")))?;
        let empty = vec![];
        let data = v.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        Ok(data
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let id = item.get("id")?.as_str()?;
                let a = item.get("attributes")?;
                // Chapter eksternal tak bisa at-home → tandai agar UI skip unduh.
                let external = a
                    .get("externalUrl")
                    .and_then(|u| u.as_str())
                    .filter(|u| !u.is_empty())
                    .map(str::to_string);
                let num = a.get("chapter").and_then(|c| c.as_str()).unwrap_or("");
                let vol = a.get("volume").and_then(|c| c.as_str()).unwrap_or("");
                let name = a.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let mut title = if num.is_empty() {
                    "Oneshot".to_string()
                } else {
                    format!("Chapter {num}")
                };
                if !vol.is_empty() {
                    title = format!("Vol. {vol} · {title}");
                }
                if !name.is_empty() {
                    title = format!("{title} — {name}");
                }
                let language = a
                    .get("translatedLanguage")
                    .and_then(|l| l.as_str())
                    .filter(|l| !l.is_empty())
                    .map(str::to_string);
                Some(Chapter {
                    id: id.to_string(),
                    content_id: manga_id.to_string(),
                    title,
                    order: base_offset + i as u32 + 1,
                    is_external: external.is_some(),
                    external_url: external,
                    language,
                })
            })
            .collect())
    }

    /// Judul: `title.en` → nilai non-kosong pertama → `altTitles` → default.
    fn md_title(attrs: &serde_json::Value) -> String {
        if let Some(map) = attrs.get("title").and_then(|t| t.as_object()) {
            if let Some(en) = map.get("en").and_then(|v| v.as_str()) {
                if !en.trim().is_empty() {
                    return en.to_string();
                }
            }
            for (_, v) in map {
                if let Some(s) = v.as_str() {
                    if !s.trim().is_empty() {
                        return s.to_string();
                    }
                }
            }
        }
        if let Some(arr) = attrs.get("altTitles").and_then(|v| v.as_array()) {
            for entry in arr {
                if let Some(map) = entry.as_object() {
                    if let Some(en) = map.get("en").and_then(|v| v.as_str()) {
                        if !en.trim().is_empty() {
                            return en.to_string();
                        }
                    }
                    for (_, v) in map {
                        if let Some(s) = v.as_str() {
                            if !s.trim().is_empty() {
                                return s.to_string();
                            }
                        }
                    }
                }
            }
        }
        "(tanpa judul)".to_string()
    }

    fn parse_mangadex(body: &str) -> Result<Vec<ContentModel>, AppError> {
        let v: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| AppError::Network(format!("mangadex json: {e}")))?;
        let empty = vec![];
        let data = v.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        let mut out = Vec::new();
        for item in data {
            let id = item.get("id").and_then(|i| i.as_str()).unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            let attrs = item.get("attributes");
            let title = attrs
                .map(Self::md_title)
                .unwrap_or_else(|| "(tanpa judul)".to_string());
            let cover = item
                .get("relationships")
                .and_then(|r| r.as_array())
                .map(|rels| {
                    rels.iter()
                        .find(|r| r.get("type").and_then(|t| t.as_str()) == Some("cover_art"))
                        .and_then(|r| {
                            r.get("attributes")
                                .and_then(|a| a.get("fileName").and_then(|f| f.as_str()))
                        })
                        .map(|f| MANGADEX_COVER_TPL.replace("{id}", id).replace("{file}", f))
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            // Field config 1:1 (`api.list.fields`): originalLanguage,
            // lastChapter→pageCount, updatedAt/createdAt.
            let language = attrs
                .and_then(|a| a.get("originalLanguage"))
                .and_then(|l| l.as_str())
                .filter(|l| !l.is_empty())
                .map(str::to_string);
            let page_count = attrs
                .and_then(|a| a.get("lastChapter"))
                .and_then(|c| c.as_str())
                .and_then(|c| c.parse::<u32>().ok());
            let upload_date = attrs
                .and_then(|a| a.get("updatedAt").or_else(|| a.get("createdAt")))
                .and_then(|d| d.as_str())
                .filter(|d| !d.is_empty())
                .map(str::to_string);
            out.push(ContentModel {
                id: id.to_string(),
                title,
                cover_url: cover,
                source_id: "mangadex".to_string(),
                upload_date,
                page_count,
                language,
                tags: Vec::new(),
                available_languages: Vec::new(),
                description: None,
                rating: None,
                favorites: None,
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
        Ok(self.fetch_list_with_next(url, config).await?.0)
    }

    /// List + link halaman berikut (cursor token; `None` bila habis/tak ada).
    pub async fn fetch_list_with_next(
        &self,
        url: &str,
        config: &SourceConfig,
    ) -> Result<(Vec<ContentModel>, Option<String>), AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        let items = Self::parse_list(&html, config)?;
        let next = if config.pagination_next.trim().is_empty() {
            None
        } else {
            extract_next_url(&html, &config.base_url, &config.pagination_next)
        };
        Ok((items, next))
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
            let mut cover = config.absolutize(&extract(item, &cover_sel, &config.cover));
            // Enrichment ala `EHentaiScraperAdapter` mobile: cover acap di
            // `style="background:...url(...)"` (`.glthumb div`), bukan `<img>`.
            if cover.is_empty() {
                cover = config.absolutize(&style_cover_url(item));
            }
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
            // Fallback tag bahasa baris (mobile: `.gt[title^="language:"]`).
            .or_else(|| row_language(item))
            .or_else(|| config.default_language.clone());
            out.push(ContentModel {
                id,
                title,
                cover_url: cover,
                source_id: config.source_id.clone(),
                upload_date: None,
                page_count,
                language,
                tags: Vec::new(),
                available_languages: Vec::new(),
                description: None,
                rating: None,
                favorites: None,
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
        Self::parse_detail(&html, url, config)
    }

    /// Parse detail dari HTML (murni, tanpa network).
    fn parse_detail(
        html: &str,
        url: &str,
        config: &SourceConfig,
    ) -> Result<ContentModel, AppError> {
        let doc = Html::parse_document(html);
        let root = doc.root_element();
        let tmap = if config.detail_title.selector.is_empty() {
            &config.title
        } else {
            &config.detail_title
        };
        let cmap = if config.detail_cover.selector.is_empty() {
            &config.cover
        } else {
            &config.detail_cover
        };
        let title_sel = Self::sel(&tmap.selector)?;
        let cover_sel = Self::sel(&cmap.selector)?;
        let title = extract(root, &title_sel, tmap);
        let title = if title.is_empty() {
            "(tanpa judul)".to_string()
        } else {
            title
        };
        let cover = config.absolutize(&extract(root, &cover_sel, cmap));
        // pageCount + language detail config-driven (mobile `toDetail`).
        let page_count = if config.detail_page_count.selector.trim().is_empty() {
            None
        } else {
            let sel = Self::sel(&config.detail_page_count.selector)?;
            let raw = extract(root, &sel, &config.detail_page_count);
            raw.split(|c: char| !c.is_ascii_digit())
                .find(|x| !x.is_empty())
                .and_then(|x| x.parse::<u32>().ok())
        };
        let language = if config.detail_language.selector.trim().is_empty() {
            None
        } else {
            let sel = Self::sel(&config.detail_language.selector)?;
            normalize_lang(&extract(root, &sel, &config.detail_language))
        }
        // Fallback tag `td_language:x` (mobile `_normalizeEhentaiTags`).
        .or_else(|| detail_language_tag(&doc))
        .or_else(|| config.default_language.clone());
        let description = if config.detail_description.selector.trim().is_empty() {
            None
        } else {
            let sel = Self::sel(&config.detail_description.selector)?;
            let raw = extract(root, &sel, &config.detail_description);
            (!raw.trim().is_empty()).then_some(raw.trim().to_string())
        };
        // Genre → `Tag(id:0, type:tag, count:0)` 1:1 `_resolveTags`.
        let tags = if config.detail_genres.selector.trim().is_empty() {
            Vec::new()
        } else {
            let sel = Self::sel(&config.detail_genres.selector)?;
            extract_all(root, &sel, &config.detail_genres)
                .into_iter()
                .map(|name| crate::domain::Tag {
                    id: "0".to_string(),
                    name,
                    tag_type: "tag".to_string(),
                    count: 0,
                })
                .collect()
        };
        Ok(ContentModel {
            id: Self::item_id(url),
            title,
            cover_url: cover,
            source_id: config.source_id.clone(),
            upload_date: None,
            page_count,
            language,
            tags,
            available_languages: Vec::new(),
            description,
            rating: None,
            favorites: None,
        })
    }

    pub async fn fetch_chapters(
        &self,
        url: &str,
        content_id: &str,
        config: &SourceConfig,
    ) -> Result<Vec<Chapter>, AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        Self::parse_chapters(&html, url, content_id, config)
    }

    /// Parse daftar chapter dari HTML (murni, tanpa network).
    fn parse_chapters(
        html: &str,
        url: &str,
        content_id: &str,
        config: &SourceConfig,
    ) -> Result<Vec<Chapter>, AppError> {
        // E-Hentai: Part chapters dari paginasi galeri (mobile `_buildPartChapters`).
        if config.source_id == "ehentai" {
            return Self::ehentai_part_chapters(html, url, content_id);
        }
        // Selector chapter kosong = sumber single-chapter.
        if config.chapter_selector.trim().is_empty() {
            return Ok(vec![Chapter {
                id: format!("{content_id}-1"),
                content_id: content_id.to_string(),
                title: "Baca".to_string(),
                order: 1,
                is_external: false,
                external_url: Some(config.absolutize(url)),
                language: None,
            }]);
        }
        let doc = Html::parse_document(html);
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
                language: None,
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
                // Scraper: bahasa per-bab tak diketahui (beda dengan MD
                // `translatedLanguage`) → lane "unknown" ala mobile.
                language: None,
            })
            .collect())
    }

    /// Part chapters E-Hentai: galeri `?p=N` jadi Part 1..N+1.
    /// Reader lazy per `/s/` butuh Fase 5; daftar part kini benar dulu.
    fn ehentai_part_chapters(
        html: &str,
        url: &str,
        content_id: &str,
    ) -> Result<Vec<Chapter>, AppError> {
        if !content_id.contains('/') {
            return Ok(vec![Chapter {
                id: format!("{content_id}-1"),
                content_id: content_id.to_string(),
                title: "Baca".to_string(),
                order: 1,
                is_external: false,
                external_url: Some(url.to_string()),
                language: None,
            }]);
        }
        let doc = Html::parse_document(html);
        let mut max_page = 0u32;
        if let Ok(sel) = Selector::parse("a[href*=\"?p=\"]") {
            for a in doc.select(&sel) {
                let href = a.value().attr("href").unwrap_or("");
                if let Some((_, q)) = href.split_once('?') {
                    for pair in q.split('&') {
                        if let Some(v) = pair.strip_prefix("p=") {
                            if let Ok(n) = v.parse::<u32>() {
                                max_page = max_page.max(n);
                            }
                        }
                    }
                }
            }
        }
        let base = url.split_once('?').map(|(b, _)| b).unwrap_or(url);
        Ok((0..=max_page)
            .map(|p| Chapter {
                id: format!("{content_id}?p={p}"),
                content_id: content_id.to_string(),
                title: format!("Part {}", p + 1),
                order: p + 1,
                is_external: false,
                external_url: Some(format!("{base}?p={p}")),
                language: None,
            })
            .collect())
    }

    pub async fn fetch_page_images(
        &self,
        url: &str,
        config: &SourceConfig,
    ) -> Result<Vec<String>, AppError> {
        let html = self.http.get(url, &config.source_id).await?;
        Self::parse_page_images(&html, config)
    }

    /// Parse URL gambar halaman dari HTML (murni, tanpa network).
    fn parse_page_images(html: &str, config: &SourceConfig) -> Result<Vec<String>, AppError> {
        let doc = Html::parse_document(html);
        let sel = Self::sel(&config.page_selector)?;
        let attr = config
            .page_attr
            .clone()
            .unwrap_or_else(|| "src".to_string());
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
            "attributes": {
                "title": {"en": "Sample Manga"},
                "altTitles": [{"ja-ro": "Sanpuru Manga"}],
                "originalLanguage": "ja",
                "lastChapter": "42",
                "updatedAt": "2026-09-01T00:00:00+00:00"
            },
            "relationships": [
                {"type": "author", "id": "a1"},
                {"type": "cover_art", "attributes": {"fileName": "cover1.jpg"}}
            ]
        }, {
            "id": "manga-2",
            "attributes": {
                "title": {},
                "altTitles": [{"en": "Alt Title Manga"}],
                "originalLanguage": "en",
                "lastChapter": null,
                "createdAt": "2026-08-01T00:00:00+00:00"
            },
            "relationships": []
        }]
    }"#;

    fn nhentai_shaped() -> SourceConfig {
        SourceConfig {
            source_id: "nhentai-test".to_string(),
            base_url: "https://nhentai.net".to_string(),
            list_path: "/".to_string(),
            home_path: "/".to_string(),
            home_page_path: String::new(),
            pagination_next: String::new(),
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
            detail_page_count: FieldMap::default(),
            detail_description: FieldMap::default(),
            detail_genres: FieldMap::default(),
            detail_language: FieldMap::default(),
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
        // Murni: parse HTML fixture langsung (tanpa socket lokal).
        let items = GenericScraperAdapter::parse_list(GALLERY_HTML, &nhentai_shaped()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "12345");
        assert_eq!(items[0].title, "Judul Satu");
        assert_eq!(items[0].cover_url, "https://t.example/1t.webp");
        assert_eq!(items[1].cover_url, "https://t.example/2t.webp");
    }

    fn mangadex_configured(
        http: HttpClientManager,
        base: &str,
        search_tpl: &str,
    ) -> GenericRestAdapter {
        use crate::data::datasources::config::{ApiSection, SourceFile};
        use std::collections::HashMap;
        let mut endpoints = HashMap::new();
        endpoints.insert(
            "search".to_string(),
            serde_json::Value::String(search_tpl.to_string()),
        );
        let file = SourceFile {
            source: "mangadex".to_string(),
            version: "test".to_string(),
            base_url: base.to_string(),
            default_language: String::new(),
            api: Some(ApiSection {
                enabled: true,
                api_base: base.to_string(),
                endpoints,
                detail: serde_json::Value::Null,
                images: serde_json::Value::Null,
                query_rules: serde_json::Value::Null,
            }),
            network: None,
            scraper: None,
            asset_hosts: std::collections::HashMap::new(),
            navigation: serde_json::Value::Null,
            search_form: serde_json::Value::Null,
            ui: serde_json::Value::Null,
            avatar_base_url: None,
            language_tag_map: std::collections::HashMap::new(),
        };
        GenericRestAdapter::new(http).with_source_file(file)
    }

    #[test]
    fn raw_overrides_all_template_values_single_query() {
        // Bug contentRating: raw satu nilai harus ganti SEMUA default
        // template (mobile `merged[key] = value`), bukan hanya kemunculan 1.
        let http = HttpClientManager::without_proxy().unwrap();
        let tpl =
            "/manga?contentRating[]=safe&contentRating[]=suggestive&limit=100&offset={offset}";
        let adapter = mangadex_configured(http, "https://api.mangadex.org", tpl);
        let url = adapter.md_endpoint_raw("search", "contentRating[]=erotica", "0", tpl);
        assert_eq!(url.matches("contentRating[]=").count(), 1, "{url}");
        assert!(url.contains("contentRating[]=erotica"), "{url}");
        assert!(url.contains("limit=100"), "{url}");
    }

    #[test]
    fn raw_param_fragment_appends_literally() {
        // Sort `rawParam=order[x]=y` ditempel utuh ala mobile
        // (`_rebuildUrlWithQueryParams`), bukan `rawParam=...`.
        let http = HttpClientManager::without_proxy().unwrap();
        let tpl = "/manga?limit=100&offset={offset}";
        let adapter = mangadex_configured(http, "https://api.mangadex.org", tpl);
        let url = adapter.md_endpoint_raw("search", "rawParam=order[rating]=desc", "0", tpl);
        assert!(url.contains("order[rating]=desc"), "{url}");
        assert!(!url.contains("rawParam="), "{url}");
    }

    #[test]
    fn query_rules_ensure_and_enforce() {
        use serde_json::json;
        let rules = json!({
            "ensureParams": {"hasAvailableChapters": "true"},
            "enforceMultiValueParams": {"availableTranslatedLanguage[]": []},
            "ensureMultiValueParamsIfMissing": {"contentRating[]": ["safe"]}
        });
        let url =
            GenericRestAdapter::apply_query_rules("https://x.example/manga?limit=5", Some(rules));
        assert!(url.contains("hasAvailableChapters=true"), "{url}");
        assert!(url.contains("contentRating[]=safe"), "{url}");
    }

    /// Fixture config MangaDex — salinan bagian relevan `mangadex-config.json`
    /// `kuron-extensions` 1.1.10 (`api.endpoints.search` + `api.queryRules.search`).
    /// Config asli TIDAK dibundel (config-driven → dari Ekstensi), jadi
    /// paritas diuji deterministik lewat fixture ini.
    const MANGADEX_FIXTURE: &str = r#"{
        "source": "mangadex",
        "version": "1.1.10",
        "baseUrl": "https://api.mangadex.org",
        "api": {
            "enabled": true,
            "endpoints": {
                "search": "/manga?title={query}&limit=100&offset={offset}&includes[]=cover_art&includes[]=author&includes[]=artist&contentRating[]=erotica&contentRating[]=pornographic&contentRating[]=suggestive&contentRating[]=safe&hasAvailableChapters=true"
            },
            "queryRules": {
                "search": {
                    "enforceMultiValueParams": {"availableTranslatedLanguage[]": []},
                    "ensureParams": {"hasAvailableChapters": "true"}
                }
            }
        }
    }"#;

    /// Adapter REST dari fixture config (temp dir ditulis → dibaca → dihapus).
    fn fixture_mangadex_adapter(tag: &str) -> GenericRestAdapter {
        use crate::data::datasources::config::SourceConfigs;
        let dir = std::env::temp_dir().join(format!("kuron-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mangadex-config.json"), MANGADEX_FIXTURE).unwrap();
        let cfgs = SourceConfigs::load_dir(&dir).unwrap();
        let file = cfgs.get("mangadex").expect("mangadex fixture").clone();
        let _ = std::fs::remove_dir_all(&dir);
        let http = HttpClientManager::without_proxy().unwrap();
        GenericRestAdapter::new(http).with_source_file(file)
    }

    #[test]
    fn user_filter_query_builds_full_mangadex_url() {
        // Paritas mobile untuk query filter nyata user (4 excludedTags +
        // mode OR + 5 originalLanguage + 2 availableTranslated + order).
        //
        // Regresi yang dijaga: dulu merge raw pakai `retain(ek != k)` per
        // nilai → kunci multi-value tinggal NILAI TERAKHIR
        // (`originalLanguage[]=zh-hk` saja). Live API: URL itu hanya
        // mengembalikan manga zh-hk (total 263), sedangkan URL di bawah
        // (utuh, ala mobile) total 43.304 — inilah "data beda" mobile vs
        // desktop. Sekarang URL wajib EKSAK sama dengan hasil mobile.
        let adapter = fixture_mangadex_adapter("md-parity");
        let raw = "excludedTags%5B%5D=5920b825-4181-4a17-beeb-9918b0ff7a30&excludedTags%5B%5D=a3c67850-4684-404e-9b7f-c69850ee5da6&excludedTags%5B%5D=2d1f5d56-a1e5-4d0d-a961-2193588b08ec&excludedTags%5B%5D=ddefd648-5140-4e5f-ba18-4eca4071d19b&excludedTagsMode=OR&originalLanguage%5B%5D=id&originalLanguage%5B%5D=en&originalLanguage%5B%5D=ja&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=zh-hk&availableTranslatedLanguage%5B%5D=id&availableTranslatedLanguage%5B%5D=en&order[latestUploadedChapter]=desc";
        let url = adapter.md_endpoint_raw("search", raw, "0", "https://api.mangadex.org/manga");
        let expected = concat!(
            "https://api.mangadex.org/manga?limit=100&offset=0",
            "&includes[]=cover_art&includes[]=author&includes[]=artist",
            "&contentRating[]=erotica&contentRating[]=pornographic",
            "&contentRating[]=suggestive&contentRating[]=safe",
            "&hasAvailableChapters=true",
            "&excludedTags[]=5920b825-4181-4a17-beeb-9918b0ff7a30",
            "&excludedTags[]=a3c67850-4684-404e-9b7f-c69850ee5da6",
            "&excludedTags[]=2d1f5d56-a1e5-4d0d-a961-2193588b08ec",
            "&excludedTags[]=ddefd648-5140-4e5f-ba18-4eca4071d19b",
            "&excludedTagsMode=OR",
            "&originalLanguage[]=id&originalLanguage[]=en",
            "&originalLanguage[]=ja&originalLanguage[]=zh",
            "&originalLanguage[]=zh-hk",
            "&availableTranslatedLanguage[]=id",
            "&availableTranslatedLanguage[]=en",
            "&order[latestUploadedChapter]=desc"
        );
        assert_eq!(url, expected);
    }

    #[test]
    fn rest_parses_mangadex_fixture() {
        // Murni: parse JSON fixture langsung (tanpa socket lokal).
        let items = GenericRestAdapter::parse_mangadex(MANGADEX_JSON).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "manga-1");
        assert_eq!(items[0].title, "Sample Manga");
        assert_eq!(
            items[0].cover_url,
            "https://mangadex.org/covers/manga-1/cover1.jpg.512.jpg"
        );
        assert_eq!(items[0].language.as_deref(), Some("ja"));
        assert_eq!(items[0].page_count, Some(42));
        assert_eq!(
            items[0].upload_date.as_deref(),
            Some("2026-09-01T00:00:00+00:00")
        );
        // Judul fallback altTitles; tanpa cover → string kosong.
        assert_eq!(items[1].title, "Alt Title Manga");
        assert!(items[1].cover_url.is_empty());
        assert_eq!(items[1].language.as_deref(), Some("en"));
        assert_eq!(items[1].page_count, None);
        assert_eq!(
            items[1].upload_date.as_deref(),
            Some("2026-08-01T00:00:00+00:00")
        );
    }

    #[test]
    fn mangadex_chapters_carry_language_and_external_flag() {
        // Murni: feed chapter 3 bahasa (1 eksternal) → grouping UI.
        let body = r#"{"data": [
            {"id": "ch-en", "attributes": {"chapter": "12", "volume": "2",
                "title": "Storm", "translatedLanguage": "en", "externalUrl": null}},
            {"id": "ch-id", "attributes": {"chapter": "12", "volume": null,
                "title": "", "translatedLanguage": "id", "externalUrl": null}},
            {"id": "ch-ext", "attributes": {"chapter": "13", "volume": null,
                "title": "", "translatedLanguage": "en",
                "externalUrl": "https://luar.example/baca"}}
        ]}"#;
        let items = GenericRestAdapter::parse_mangadex_chapters(body, "m1", 0).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].language.as_deref(), Some("en"));
        assert_eq!(items[0].title, "Vol. 2 · Chapter 12 — Storm");
        assert!(!items[0].is_external);
        assert_eq!(items[1].language.as_deref(), Some("id"));
        assert_eq!(items[1].title, "Chapter 12");
        assert!(items[2].is_external);
        assert_eq!(
            items[2].external_url.as_deref(),
            Some("https://luar.example/baca")
        );
    }

    #[test]
    fn md_chapters_keep_feed_order_not_number_sort() {
        // Regresi user: urutan MD "ngaco" vs mobile. Mobile TAK sorting
        // (tanpa field order) — tampil urut feed API (`order[chapter]=desc`).
        // `order` = posisi feed 1-based (+offset load-more), BUKAN parse
        // nomor chapter (gagal di desimal/sufiks/oneshot → campur skema).
        let body = r#"{"data": [
            {"id": "ch-13", "attributes": {"chapter": "13", "volume": null,
                "title": "", "translatedLanguage": "en", "externalUrl": null}},
            {"id": "ch-125", "attributes": {"chapter": "12.5", "volume": null,
                "title": "", "translatedLanguage": "en", "externalUrl": null}},
            {"id": "ch-12", "attributes": {"chapter": "12", "volume": null,
                "title": "", "translatedLanguage": "en", "externalUrl": null}},
            {"id": "ch-one", "attributes": {"chapter": null, "volume": null,
                "title": "", "translatedLanguage": "en", "externalUrl": null}}
        ]}"#;
        let items = GenericRestAdapter::parse_mangadex_chapters(body, "m1", 0).unwrap();
        let ids: Vec<&str> = items.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["ch-13", "ch-125", "ch-12", "ch-one"]);
        let orders: Vec<u32> = items.iter().map(|c| c.order).collect();
        assert_eq!(orders, vec![1, 2, 3, 4]);
        // Halaman load-more lanjut (offset=100 → 101..).
        let more = GenericRestAdapter::parse_mangadex_chapters(body, "m1", 100).unwrap();
        assert_eq!(more[0].order, 101);
    }

    #[test]
    fn md_chapters_url_upserts_offset_for_load_more() {
        // Murni: offset load-more ala mobile `_upsertQueryParam`.
        let u = GenericRestAdapter::md_chapters_url(
            MANGADEX_CHAPTERS,
            MANGADEX_API,
            "m1",
            Some("id"),
            Some(100),
        );
        assert!(u.contains("translatedLanguage[]=id"), "{u}");
        assert!(u.contains("limit=100"), "{u}");
        assert!(u.ends_with("offset=100"), "{u}");
        // Halaman pertama: tanpa offset; None/0 sama.
        for off in [None, Some(0)] {
            let u0 = GenericRestAdapter::md_chapters_url(
                MANGADEX_CHAPTERS,
                MANGADEX_API,
                "m1",
                Some("en"),
                off,
            );
            assert!(!u0.contains("offset="), "{u0}");
        }
    }

    #[cfg(feature = "live-tests")]
    #[test]
    fn live_ehentai_cursor_pagination() {
        use std::collections::HashSet;
        let http = HttpClientManager::new().unwrap();
        let engine = GenericScraperAdapter::new(http);
        let cfg = ehentai_config();
        let (items1, next) = tauri::async_runtime::block_on(
            engine.fetch_list_with_next(&cfg.home_url_page(1), &cfg),
        )
        .unwrap();
        assert!(!items1.is_empty(), "home EH kosong (CF/cookie?)");
        let next_url = next.expect("halaman 1 EH tanpa link next");
        assert!(next_url.contains("next="), "{next_url}");

        // Submit form `raw:` (4c): f_search Montenegro site.
        let raw_url = cfg.search_url_raw("raw:f_search=english", 1);
        let (raw_items, _) =
            tauri::async_runtime::block_on(engine.fetch_list_with_next(&raw_url, &cfg)).unwrap();
        assert!(!raw_items.is_empty(), "raw f_search kosong: {raw_url}");
        let (items2, _) =
            tauri::async_runtime::block_on(engine.fetch_list_with_next(&next_url, &cfg)).unwrap();
        assert!(!items2.is_empty(), "halaman 2 EH kosong");
        let ids1: HashSet<&str> = items1.iter().map(|c| c.id.as_str()).collect();
        assert!(
            items2.iter().any(|c| !ids1.contains(c.id.as_str())),
            "halaman 2 duplikat halaman 1 (?page= diabaikan situs)"
        );
    }
    #[cfg(feature = "live-tests")]
    #[test]
    fn live_mangadex_search_parses() {
        // NHentai/Hitomi/E-H live 403 dari network tanpa cookie CF
        // (risiko spec §7 — mitigasi: cookie harvest Fase 2 lanjutan).
        let http = HttpClientManager::new().unwrap();
        let rest = GenericRestAdapter::new(http);
        let items = tauri::async_runtime::block_on(rest.search_mangadex("test", 1)).unwrap();
        assert!(!items.is_empty(), "live search mengembalikan item");
        assert!(!items[0].id.is_empty());
        assert!(!items[0].title.is_empty());
        let detail =
            tauri::async_runtime::block_on(rest.get_mangadex_detail(&items[0].id)).unwrap();
        assert_eq!(detail.id, items[0].id);
        // Home (allGalleries) + chapter + at-home 1:1 config.
        let home = tauri::async_runtime::block_on(rest.search_mangadex("", 1)).unwrap();
        assert!(!home.is_empty(), "allGalleries kosong");
        assert!(home[0].language.is_some(), "bahasa hilang: {:?}", home[0]);
        let chapters =
            tauri::async_runtime::block_on(rest.chapters_mangadex(&items[0].id, None, None))
                .unwrap();
        assert!(!chapters.is_empty(), "chapter kosong untuk {}", items[0].id);
        let internal = chapters.iter().find(|c| !c.is_external);
        if let Some(ch) = internal {
            let pages =
                tauri::async_runtime::block_on(rest.pages_mangadex_at_home(&ch.id)).unwrap();
            assert!(!pages.is_empty(), "at-home kosong untuk {}", ch.id);
            assert!(pages[0].starts_with("http"), "{}", pages[0]);
        }
        // Submit form `raw:` (4c).
        let raw_items =
            tauri::async_runtime::block_on(rest.search_mangadex("raw:title=naruto", 1)).unwrap();
        assert!(!raw_items.is_empty(), "raw title=naruto kosong");
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
    tag_tpl: String,
    detail_tpl: String,
    related_tpl: String,
    thumb_host: String,
    img_host: String,
    avatar_base: String,
    lang_map: std::collections::HashMap<String, String>,
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
            search_tpl: ep(
                "search",
                "/api/v2/search?query={query}&sort={sort}&page={page}",
            ),
            all_tpl: ep("allGalleries", "/api/v2/galleries?page={page}"),
            tag_tpl: ep(
                "tagSearch",
                "/api/v2/galleries/tagged?tag_id={tagId}&page={page}",
            ),
            detail_tpl: ep("galleryDetail", "/api/v2/galleries/{id}"),
            related_tpl: ep("related", "/api/v2/galleries/{id}/related"),
            thumb_host: host("thumbnail", "https://t.nhentai.net"),
            img_host: host("image", "https://i.nhentai.net"),
            avatar_base: cfg
                .avatar_base_url
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "https://i3.nhentai.net".to_string()),
            lang_map: if cfg.language_tag_map.is_empty() {
                default_language_tag_map()
            } else {
                cfg.language_tag_map.clone()
            },
            attempts,
            retry_delay_ms,
        }
    }

    /// Bahasa dari `tag_ids` via `languageTagMap` config (mobile
    /// `_resolveLanguageTagMap`): cocok PERTAMA urut array, lewati
    /// "translated", None bila tak cocok (tanpa default).
    fn lang_of_ids(&self, item: &serde_json::Value) -> Option<String> {
        let arr = item.get("tag_ids")?.as_array()?;
        for v in arr {
            let Some(id) = v
                .as_u64()
                .map(|n| n.to_string())
                .or_else(|| v.as_str().map(str::to_string))
            else {
                continue;
            };
            let Some(name) = self.lang_map.get(&id).map(|s| s.trim()) else {
                continue;
            };
            if name.is_empty() || name.eq_ignore_ascii_case("translated") {
                continue;
            }
            if let Some(code) = normalize_lang(name) {
                return Some(code);
            }
        }
        None
    }

    async fn fetch(&self, url: &str) -> Result<serde_json::Value, AppError> {
        self.limiter.wait("nhentai.net").await;
        let body = self
            .http
            .get_with_retry(url, "nhentai", self.attempts, self.retry_delay_ms)
            .await?;
        serde_json::from_str(&body).map_err(|e| AppError::Network(format!("nhentai json: {e}")))
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
                item.get("title")?
                    .get("english")?
                    .as_str()
                    .map(String::from)
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
            language: self.lang_of_ids(item),
            tags: Vec::new(),
            available_languages: Vec::new(),
            description: None,
            rating: None,
            favorites: None,
        })
    }

    /// URL search murni: payload `raw:k=v&..` dari form filter diurai
    /// (ala mobile `_searchRaw`) — `raw:` harfiah JANGAN dikirim sebagai
    /// kueri (API membalas 0 hasil). `tag_id` → endpoint `tagSearch`.
    fn search_url(&self, query: &str, page: u32) -> String {
        let page = page.max(1).to_string();
        let fill = crate::data::datasources::config::fill_url;
        let plain = |q: &str| {
            if q.trim().is_empty() {
                // API menolak query kosong (400) → home feed pakai allGalleries.
                fill(&self.all_tpl.clone(), &[("page", &page)])
            } else {
                fill(
                    &self.search_tpl.clone(),
                    &[
                        ("query", &q.replace(' ', "+")),
                        ("sort", "popular"),
                        ("page", &page),
                    ],
                )
            }
        };
        let raw = match query.strip_prefix("raw:") {
            Some(r) => r,
            None => return plain(query),
        };
        let pairs = parse_raw_params(raw);
        if let Some(tag_id) = raw_first(&pairs, &["tag_id", "tagId"]) {
            if !tag_id.trim().is_empty() {
                return fill(
                    &self.tag_tpl.clone(),
                    &[
                        ("tagId", tag_id.trim()),
                        ("tag_id", tag_id.trim()),
                        ("page", &page),
                    ],
                );
            }
        }
        let q = raw_query_value(&pairs, "query");
        if q.trim().is_empty() {
            return fill(&self.all_tpl.clone(), &[("page", &page)]);
        }
        let sort = raw_first(&pairs, &["sort", "order"])
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "popular".to_string());
        fill(
            &self.search_tpl.clone(),
            &[
                ("query", &q.replace(' ', "+")),
                ("sort", &sort),
                ("page", &page),
            ],
        )
    }

    pub async fn search(&self, query: &str, page: u32) -> Result<Vec<ContentModel>, AppError> {
        let url = self.search_url(query, page);
        let v = self.fetch(&url).await?;
        Ok(Self::items_of(&v)
            .iter()
            .filter_map(|i| self.to_model(i))
            .collect())
    }

    pub async fn detail(&self, id: &str) -> Result<ContentModel, AppError> {
        let url =
            crate::data::datasources::config::fill_url(&self.detail_tpl.clone(), &[("id", id)]);
        let v = self.fetch(&url).await?;
        let title = v
            .get("title")
            .and_then(|t| {
                t.get("pretty")
                    .or_else(|| t.get("english"))
                    .or_else(|| t.get("japanese"))
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
            tags: parse_nhentai_tags(&v),
            available_languages: Vec::new(),
            description: None,
            rating: None,
            favorites: v.get("num_favorites").and_then(|n| n.as_i64()),
        })
    }

    /// Galeri terkait (endpoint `related` config; live: `{result: [...]}`).
    pub async fn related(&self, gallery_id: &str) -> Result<Vec<ContentModel>, AppError> {
        let url = crate::data::datasources::config::fill_url(
            &self.related_tpl.clone(),
            &[("id", gallery_id)],
        );
        let v = self.fetch(&url).await?;
        Ok(Self::items_of(&v)
            .iter()
            .filter_map(|item| self.to_model(item))
            .collect())
    }

    /// Komentar galeri — embedded di respons detail `?include=comments`
    /// (live: top-level `comments: [...]`), tanpa endpoint terpisah.
    pub async fn comments(&self, gallery_id: &str) -> Result<Vec<Comment>, AppError> {
        let url = crate::data::datasources::config::fill_url(
            &self.detail_tpl.clone(),
            &[("id", gallery_id)],
        );
        let v = self.fetch(&url).await?;
        Ok(self.parse_comments(&v))
    }

    /// Avatar API sering relatif/protokol-relatif (`/avatars/…`,
    /// `//i.nhentai.net/…`, `avatars/…`) — resolve absolut 1:1 mobile
    /// (`comment_model.dart`); host dari `avatarBaseUrl` config.
    fn resolve_nh_avatar(&self, raw: &str) -> Option<String> {
        let r = raw.trim();
        if r.is_empty() {
            return None;
        }
        if let Some(rest) = r.strip_prefix("//") {
            Some(format!("https://{rest}"))
        } else if r.starts_with('/') {
            Some(format!("{}{r}", self.avatar_base))
        } else if r.starts_with("http") {
            Some(r.to_string())
        } else {
            Some(format!("{}/{r}", self.avatar_base))
        }
    }

    /// Parse `comments[]` → `Comment` (murni). Bentuk per mobile
    /// `NhentaiComment`: `{id, gallery_id, poster{username, avatar_url},
    /// body, post_date}` — toleran: kunci hilang = default mobile.
    fn parse_comments(&self, v: &serde_json::Value) -> Vec<Comment> {
        v.get("comments")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|c| {
                        let num_id = |k: &str| {
                            c.get(k).and_then(|n| {
                                n.as_u64()
                                    .map(|n| n.to_string())
                                    .or_else(|| n.as_str().map(str::to_string))
                            })
                        };
                        let poster = c.get("poster");
                        let str_of = |k: &str| {
                            poster
                                .and_then(|p| p.get(k))
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string()
                        };
                        Comment {
                            id: num_id("id").unwrap_or_else(|| "0".to_string()),
                            username: str_of("username"),
                            body: c
                                .get("body")
                                .and_then(|b| b.as_str())
                                .unwrap_or("")
                                .to_string(),
                            avatar_url: self.resolve_nh_avatar(&str_of("avatar_url")),
                            post_date: c
                                .get("post_date")
                                .and_then(|n| n.as_i64().or_else(|| n.as_u64().map(|n| n as i64))),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// URL gambar penuh satu galeri (untuk reader).
    pub async fn pages(&self, gallery_id: &str) -> Result<Vec<String>, AppError> {
        let url = crate::data::datasources::config::fill_url(
            &self.detail_tpl.clone(),
            &[("id", gallery_id)],
        );
        let v = self.fetch(&url).await?;
        Ok(v.get("pages")
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

    /// Chapter id `"{gallery}-1"` → gallery id. Terima pula URL penuh
    /// `.../g/{gallery}/` karena frontend mengirim `external_url` (kontrak
    /// umum bab) — digitnya diekstrak agar tak masuk `{id}` mentah (404).
    pub fn gallery_of_chapter(chapter_id: &str) -> &str {
        let bare = chapter_id.strip_suffix("-1").unwrap_or(chapter_id);
        if let Some((_, rest)) = bare.split_once("/g/") {
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            if end > 0 {
                return &rest[..end];
            }
        }
        bare
    }
}

#[cfg(test)]
mod nhentai_tests {
    use super::*;
    use crate::data::datasources::config::SourceFile;

    #[test]
    fn json_engine_parses_areakomik_shaped_fixture() {
        // Murni: config JSON inline (bentuk installed) + parse fixture
        // langsung (tanpa socket lokal). Base fiktif khusus absolutize URL.
        const BASE: &str = "http://ak.test";
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
            <div class="series-sinopsis">Sinopsis uji.</div>
            <div class="genre-list"><a>Aksi</a><a>Petualangan</a></div>
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
                    "description": {"selector": ".series-sinopsis"},
                    "genres": {"selector": ".genre-list a", "multi": true},
                    "coverUrl": {"selector": "meta[property='og:image']", "attribute": "content"}},
                "chapters": {"container": ".chapter-grid .chapter-row", "fields": {
                    "id": {"selector": "a.chapter-link", "attribute": "href", "transform": "slug"},
                    "title": {"selector": ".chap-num"}}}},
                "reader": {"images": {"selector": "img", "attribute": "src"}}}}"#;
        let file: crate::data::datasources::config::SourceFile = serde_json::from_str(&format!(
            r#"{{"source": "areakomik-test", "baseUrl": "{BASE}", "scraper": {scraper_json}}}"#
        ))
        .unwrap();
        let cfg = source_config_from_json(
            "areakomik-test",
            &file.base_url,
            file.scraper.as_ref().unwrap(),
            "indonesian",
        )
        .expect("pola areakomik terbaca");
        assert_eq!(cfg.list_path, "/ak-list");
        let items = GenericScraperAdapter::parse_list(LIST, &cfg).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "slug-satu");
        assert_eq!(items[0].title, "Judul Satu");
        assert_eq!(items[0].cover_url, "https://cdn.example/1.jpg");
        assert_eq!(items[0].language.as_deref(), Some("en"));
        assert_eq!(items[0].page_count, Some(24));
        let detail_url = format!("{BASE}/ak-detail");
        let detail = GenericScraperAdapter::parse_detail(DETAIL, &detail_url, &cfg).unwrap();
        assert_eq!(detail.title, "Judul Satu");
        assert_eq!(detail.cover_url, "https://cdn.example/cover.jpg");
        // Config-driven 1:1 mobile: description + genres→tags.
        assert_eq!(detail.description.as_deref(), Some("Sinopsis uji."));
        assert_eq!(detail.tags.len(), 2);
        assert_eq!(detail.tags[0].name, "Aksi");
        assert_eq!(detail.tags[1].name, "Petualangan");
        assert!(detail
            .tags
            .iter()
            .all(|t| t.tag_type == "tag" && t.count == 0));
        let chapters =
            GenericScraperAdapter::parse_chapters(DETAIL, &detail_url, "x", &cfg).unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].id, "c1");
        assert_eq!(chapters[0].title, "Chapter 1");
        let expect_ch = format!("{BASE}/chapter/c1/");
        assert_eq!(
            chapters[0].external_url.as_deref(),
            Some(expect_ch.as_str())
        );
        let pages = GenericScraperAdapter::parse_page_images(CHAPTER, &cfg).unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].contains("gudangkomik"));
    }

    #[test]
    fn raw_query_builds_scraper_and_rest_urls() {
        // Scraper E-Hentai: payload raw ganti template, page dikendalikan adapter.
        let eh = ehentai_config();
        let u = eh.search_url_raw("raw:f_search=language%3Aenglish&page=2", 1);
        assert!(u.contains("f_search=language%3Aenglish"), "{u}");
        assert!(!u.contains("page=2"), "{u}");
        assert!(u.starts_with("https://e-hentai.org/?"), "{u}");
        // REST MangaDex: merge template + raw + offset paging.
        let http = HttpClientManager::without_proxy().unwrap();
        let rest = GenericRestAdapter::new(http);
        let u = rest.md_endpoint_raw(
            "search",
            "title=naruto&status=completed",
            "0",
            MANGADEX_SEARCH,
        );
        assert!(u.contains("title=naruto"), "{u}");
        assert!(u.contains("status=completed"), "{u}");
        assert!(u.contains("offset=0"), "{u}");
        assert!(u.contains("hasAvailableChapters=true"), "{u}");
        assert!(!u.contains("title=&"), "{u}");
    }

    #[test]
    fn ehentai_next_token_extracted() {
        // Paging E-Hentai = token `?next=`, bukan `?page=` (curl bukti 2026-09-20).
        let html = r#"<html><body><div class="searchnav">
            <a id="uprev" href="/?prev=99">Prev</a>
            <a id="unext" href="/?next=4200603">Next</a></div>
            <script>var nexturl="https://e-hentai.org/?next=4200603";</script></body></html>"#;
        let next = extract_next_url(html, "https://e-hentai.org", "#unext").unwrap();
        assert_eq!(next, "https://e-hentai.org/?next=4200603");
        // Tanpa selector config: cadangan #dnext/.searchnav tetap jalan.
        let html2 = r#"<html><body><div class="searchnav"><a href="/?f_search=x&next=77">N</a></div></body></html>"#;
        let next2 = extract_next_url(html2, "https://e-hentai.org", "").unwrap();
        assert!(next2.contains("next=77"), "{next2}");
        // Fallback script bila nav tak ada.
        let html3 = r#"<html><body><script>var nexturl="/?next=55";</script></body></html>"#;
        let next3 = extract_next_url(html3, "https://e-hentai.org", "").unwrap();
        assert!(next3.contains("next=55"), "{next3}");
        // Cursor put/get/exhausted.
        let c = PaginationCursors::default();
        let k1 = PaginationCursors::home_key("ehentai", 1);
        let k2 = PaginationCursors::home_key("ehentai", 2);
        assert!(c.get(&k2).is_none());
        c.put_next(&k2, "https://e-hentai.org/?next=1".to_string());
        assert!(matches!(c.get(&k2), Some(CursorState::Next(_))));
        assert!(!matches!(c.get(&k2), Some(CursorState::Exhausted)));
        c.mark_exhausted(&k1);
        assert!(matches!(c.get(&k1), Some(CursorState::Exhausted)));
        assert!(!matches!(c.get(&k1), Some(CursorState::Next(_))));
        let _ = k1;
    }

    #[test]
    fn configs_constructible() {
        // Site config kompilasi + URL terbentuk (tuning selector lanjut live).
        let cfg = nhentai_config();
        assert!(cfg.list_url("test", 1).contains("nhentai.net/search"));
        assert!(hitomi_config().list_url("x", 1).contains("hitomi.la"));
        let eh = ehentai_config();
        assert!(eh.list_url("x", 1).contains("e-hentai.org"));
        // Nilai 1:1 `ehentai-config.json` mobile.
        assert_eq!(eh.item_selector, ".itg.gltc tr");
        assert_eq!(eh.home_path, "/?page={page}");
        // Load more home: halaman 2 WAJIB beda URL (bug 2026-09-20: ulang hal 1).
        assert!(eh.home_url_page(1).ends_with("/?page=1") || eh.home_url_page(1).ends_with("/"));
        assert!(
            eh.home_url_page(2).contains("page=2"),
            "{}",
            eh.home_url_page(2)
        );
    }

    #[test]
    fn ehentai_list_enrichment_and_parts() {
        // Cover style + bahasa .gt + part chapters ala mobile.
        let html = r#"<html><body><table class="itg gltc">
            <tr><td class="gl2c"><div class="glthumb"><div style="background:transparent url(https://ehgt.org/aa/bb.jpg) no-repeat"></div></div></td>
            <td class="gl3c glname"><a href="https://e-hentai.org/g/12345/tokenab/"><div class="glink">Galeri Satu</div></a>
            <div class="gt" title="language:english"></div></td></tr>
            </table>
            <a href="/g/12345/tokenab/?p=1">2</a></body></html>"#;
        let cfg = ehentai_config();
        let items = GenericScraperAdapter::parse_list(html, &cfg).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "12345/tokenab");
        assert_eq!(items[0].cover_url, "https://ehgt.org/aa/bb.jpg");
        assert_eq!(items[0].language.as_deref(), Some("en"));
        let parts = GenericScraperAdapter::ehentai_part_chapters(
            html,
            "https://e-hentai.org/g/12345/tokenab/",
            "12345/tokenab",
        )
        .unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].title, "Part 1");
        assert_eq!(
            parts[1].external_url.as_deref(),
            Some("https://e-hentai.org/g/12345/tokenab/?p=1")
        );
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
        // Regresi kasus user: `external_url` penuh → id numerik murni.
        assert_eq!(
            NhentaiApiAdapter::gallery_of_chapter("https://nhentai.net/g/682821/"),
            "682821"
        );
        assert_eq!(NhentaiApiAdapter::gallery_of_chapter("682821"), "682821");
        // Detail diparse dari bentuk live (tanpa network).
        let d: serde_json::Value = serde_json::from_str(DETAIL_JSON).unwrap();
        assert_eq!(d.get("title").unwrap().get("pretty").unwrap(), "Judul Rapi");
        let pages: Vec<String> = d
            .get("pages")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                format!(
                    "https://i.nhentai.net/{}",
                    p.get("path").unwrap().as_str().unwrap()
                )
            })
            .collect();
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn nhentai_related_parses_live_shaped_result() {
        // Murni: `{result: [...]}` bentuk live (id numerik, judul datar).
        let cfg: SourceFile = serde_json::from_str(
            r#"{"source": "nhentai", "api": {"apiBase": "https://nhentai.net"},
                "assetHosts": {"image": "https://i.nhentai.net", "thumbnail": "https://t.nhentai.net"}}"#,
        )
        .unwrap();
        let a = NhentaiApiAdapter::from_config(HttpClientManager::without_proxy().unwrap(), &cfg);
        let v: serde_json::Value = serde_json::from_str(
            r#"{"result": [
                {"id": 508415, "english_title": "Terkait Satu",
                 "thumbnail": "galleries/1/thumb.webp", "num_pages": 20},
                {"id": 999, "english_title": "", "thumbnail": "", "num_pages": null}
            ]}"#,
        )
        .unwrap();
        let items: Vec<_> = NhentaiApiAdapter::items_of(&v)
            .iter()
            .filter_map(|item| a.to_model(item))
            .collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "508415");
        assert_eq!(items[0].title, "Terkait Satu");
        assert_eq!(
            items[0].cover_url,
            "https://t.nhentai.net/galleries/1/thumb.webp"
        );
        assert_eq!(items[0].page_count, Some(20));
        assert_eq!(items[1].title, "");
    }

    #[test]
    fn nhentai_comments_parse_mobile_shaped_items() {
        // Murni: `comments[]` embedded (bentuk model mobile NhentaiComment).
        let v: serde_json::Value = serde_json::from_str(
            r#"{"comments": [
                {"id": 7, "gallery_id": 682821,
                 "poster": {"username": "suke", "avatar_url": "https://a.example/u.png"},
                 "body": "mantap!", "post_date": 1700000000},
                {"id": 8}
            ]}"#,
        )
        .unwrap();
        let out = test_adapter().parse_comments(&v);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "7");
        assert_eq!(out[0].username, "suke");
        assert_eq!(out[0].body, "mantap!");
        assert_eq!(
            out[0].avatar_url.as_deref(),
            Some("https://a.example/u.png")
        );
        assert_eq!(out[0].post_date, Some(1_700_000_000));
        // Entri jarang: default mobile (username/body kosong).
        assert_eq!(out[1].id, "8");
        assert_eq!(out[1].username, "");
        assert!(out[1].avatar_url.is_none());
    }

    #[test]
    fn nhentai_avatar_resolves_all_mobile_shapes() {
        // Murni 1:1 `comment_model.dart`: 4 bentuk avatar API.
        // (adapter uji tanpa `avatarBaseUrl` → fallback config bundel i3).
        let a = test_adapter();
        let r = |s: &str| a.resolve_nh_avatar(s);
        assert_eq!(
            r("//i.nhentai.net/avatars/1.png").as_deref(),
            Some("https://i.nhentai.net/avatars/1.png")
        );
        assert_eq!(
            r("/avatars/1.png").as_deref(),
            Some("https://i3.nhentai.net/avatars/1.png")
        );
        assert_eq!(
            r("avatars/1.png").as_deref(),
            Some("https://i3.nhentai.net/avatars/1.png")
        );
        assert_eq!(
            r("https://a.example/u.png").as_deref(),
            Some("https://a.example/u.png")
        );
        assert!(r("").is_none());
        assert!(r("   ").is_none());
    }

    #[test]
    fn nhentai_config_drives_lang_map_and_avatar_base() {
        // Config instal (bukan hardcode) menang: map kustom + base kustom.
        let cfg: SourceFile = serde_json::from_str(
            r#"{"source": "nhentai",
                "avatarBaseUrl": "https://av.custom.net",
                "languageTagMap": {"777": "indonesian", "888": "translated"},
                "network": {"rateLimit": {"minDelayMs": 0}}}"#,
        )
        .unwrap();
        let a = NhentaiApiAdapter::from_config(HttpClientManager::without_proxy().unwrap(), &cfg);
        assert_eq!(
            a.resolve_nh_avatar("/a/1.png").as_deref(),
            Some("https://av.custom.net/a/1.png")
        );
        // Cocok pertama urut array; "translated" dilewati.
        let v: serde_json::Value = serde_json::from_str(r#"{"tag_ids": [888, 5, 777]}"#).unwrap();
        assert_eq!(a.lang_of_ids(&v).as_deref(), Some("id"));
        // Tanpa cocok → None (mobile: tanpa default).
        let v2: serde_json::Value = serde_json::from_str(r#"{"tag_ids": [5]}"#).unwrap();
        assert!(a.lang_of_ids(&v2).is_none());
        let v3: serde_json::Value = serde_json::from_str("{}").unwrap();
        assert!(a.lang_of_ids(&v3).is_none());
        // Fallback bundel tetap untuk config tanpa map (kasus SEARCH_JSON).
        let b = test_adapter();
        let v4: serde_json::Value = serde_json::from_str(SEARCH_JSON).unwrap();
        let first = &NhentaiApiAdapter::items_of(&v4)[0];
        assert_eq!(b.lang_of_ids(first).as_deref(), Some("en"));
    }

    #[test]
    fn nhentai_tags_parse_live_shaped() {
        // Murni: `tags[]` bentuk live (kasus galeri 682821).
        let v: serde_json::Value = serde_json::from_str(
            r#"{"tags": [
                {"id": 33172, "type": "category", "name": "doujinshi",
                 "slug": "doujinshi", "url": "/category/doujinshi/", "count": 508869},
                {"id": 5, "type": "language", "name": "chinese", "count": 1},
                {"id": 6}
            ]}"#,
        )
        .unwrap();
        let tags = parse_nhentai_tags(&v);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].id, "33172");
        assert_eq!(tags[0].name, "doujinshi");
        assert_eq!(tags[0].tag_type, "category");
        assert_eq!(tags[0].count, 508869);
        assert_eq!(tags[1].name, "chinese");
    }

    #[test]
    fn md_detail_enrichment_parses_live_shaped() {
        // Murni: tags + availableTranslatedLanguages + description MD.
        let attrs: serde_json::Value = serde_json::from_str(
            r#"{"tags": [
                {"id": "tag-uuid-1", "attributes": {"name": {"en": "Action"}, "group": "genre"}},
                {"id": "tag-uuid-2", "attributes": {"name": {"ja": "冒険"}, "group": "theme"}}
            ],
            "availableTranslatedLanguages": ["en", "id"],
            "description": {"en": "Sinopsis.", "id": "Ringkasan."}}"#,
        )
        .unwrap();
        let tags = GenericRestAdapter::md_tags_of(&attrs);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].id, "tag-uuid-1");
        assert_eq!(tags[0].name, "Action");
        assert_eq!(tags[0].tag_type, "genre");
        assert_eq!(tags[1].name, "冒険");
        let langs = GenericRestAdapter::md_avail_langs_of(&attrs);
        assert_eq!(langs, vec!["en".to_string(), "id".to_string()]);
        assert_eq!(
            GenericRestAdapter::md_description_of(&attrs).as_deref(),
            Some("Sinopsis.")
        );
    }

    #[test]
    fn md_stats_parses_follows_and_rating() {
        // Murni: `$.statistics.{id}.follows` + `.rating.average` (mobile).
        let (follows, rating) = GenericRestAdapter::parse_md_stats(
            r#"{"statistics": {"m1": {"follows": 1200, "rating": {"average": 8.71}}}}"#,
            "m1",
        );
        assert_eq!(follows, Some(1200));
        assert_eq!(rating, Some(8.71));
        let (none_f, none_r) = GenericRestAdapter::parse_md_stats("{}", "m1");
        assert!(none_f.is_none() && none_r.is_none());
    }

    #[test]
    fn md_recommend_ids_skip_self_dedup_ordered() {
        // Murni: relationships[type==manga].id, buang self + dedup.
        let ids = GenericRestAdapter::parse_md_recommend_ids(
            r#"{"data": [
                {"relationships": [{"type": "manga", "id": "self"}, {"type": "manga", "id": "r1"}]},
                {"relationships": [{"type": "manga", "id": "r1"}, {"type": "manga", "id": "r2"}, {"type": "user", "id": "u9"}]}
            ]}"#,
            "self",
        );
        assert_eq!(ids, vec!["r1".to_string(), "r2".to_string()]);
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
    fn raw_filter_builds_correct_search_url() {
        // Regresi: form filter kirim `raw:query=..&sort=..`; adapter lama
        // meng-encode harfiah `raw:..` sebagai kueri → API balas 0 hasil.
        let a = test_adapter();
        let u = a.search_url("raw:query=Language%3Aenglish", 1);
        assert!(u.contains("query=Language%3Aenglish"), "{u}");
        assert!(u.contains("sort=popular"), "{u}");
        assert!(!u.contains("raw%3A"), "{u}");
        let u = a.search_url("raw:query=yuri&sort=popular-today", 2);
        assert!(u.contains("query=yuri"), "{u}");
        assert!(u.contains("sort=popular-today"), "{u}");
        assert!(u.contains("page=2"), "{u}");
        // Tap-tag (`tagQueryMapping` → `raw:tag_id=N`) pakai endpoint tagged.
        let u = a.search_url("raw:tag_id=12227", 1);
        assert!(u.contains("tagged?tag_id=12227"), "{u}");
        // Jalur lama tak berubah: teks polos + kosong → allGalleries.
        assert!(a.search_url("test", 1).contains("query=test"));
        assert!(a.search_url("", 1).contains("/api/v2/galleries?page=1"));
        // raw tanpa query (cuma sort) → allGalleries, hindari 400.
        assert!(a
            .search_url("raw:sort=date", 1)
            .contains("/api/v2/galleries?page=1"));
    }

    #[cfg(feature = "live-tests")]
    #[test]
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
        let detail = tauri::async_runtime::block_on(a.detail(&items[0].id)).unwrap();
        assert_eq!(detail.id, items[0].id);
        let pages = tauri::async_runtime::block_on(a.pages(&items[0].id)).unwrap();
        assert!(!pages.is_empty());
        assert!(pages[0].starts_with("https://i.nhentai.net/"));
        // Regresi: query kosong (home feed) pakai allGalleries, bukan 400.
        let home = tauri::async_runtime::block_on(a.search("", 1)).unwrap();
        assert!(!home.is_empty());
    }
}
