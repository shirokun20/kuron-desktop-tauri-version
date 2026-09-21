//! Content commands: feed halaman utama (mock Fase 0, real Fase 2).
//! Search/detail/chapters/pages: kontrak baru Fase 2, mock kini.
//! Repo di-resolve dari `AppState` (DI), bukan dikonstruksi di sini.

use tauri::State;

use crate::{
    application::{
        GetChaptersUseCase, GetCommentsUseCase, GetContentDetailUseCase, GetHomeFeedUseCase,
        GetPageImagesUseCase, GetRelatedContentUseCase, SearchContentUseCase,
    },
    core::AppState,
    domain::{Chapter, Comment, Content, PageImageResult, SearchFilter},
};

#[tauri::command]
pub async fn cmd_home_feed(
    state: State<'_, AppState>,
    source: Option<String>,
    page: Option<u32>,
) -> Result<Vec<Content>, String> {
    let page = page.unwrap_or(1).max(1);
    let repo = match source.as_deref() {
        None | Some("semua") => state.content_repo.clone(),
        Some(id) => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
    };
    GetHomeFeedUseCase::new(repo)
        .execute(page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_search(
    state: State<'_, AppState>,
    filter: SearchFilter,
) -> Result<Vec<Content>, String> {
    // Search ikut sumber aktif seperti home_feed (dulu mock-only — bug 4c).
    let repo = match filter.source_id.as_deref() {
        Some(id) if !id.is_empty() && id != "semua" => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
        _ => state.content_repo.clone(),
    };
    SearchContentUseCase::new(repo)
        .execute(filter)
        .await
        .map_err(|e| e.to_string())
}

/// Definisi `searchForm` satu sumber (field unik per situs, mobile
/// `DynamicFormSearchUI`); `dataSources` endpoint di-resolve jadi `options`
/// (MangaDex `/manga/tag` → `attributes.name.en`). Nilai `params.*` ikut
/// urutan JSON config (teks dulu di frontend, sisanya 1:1 config).
#[tauri::command]
pub async fn cmd_search_form(
    state: State<'_, AppState>,
    source: String,
) -> Result<serde_json::Value, String> {
    use crate::data::datasources::config::SourceConfigs;
    let id = source.to_lowercase();
    let overlay = SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &state.ext_dir)
        .map_err(|e| e.to_string())?;
    let file = overlay
        .get(&id)
        .ok_or_else(|| format!("sumber '{id}' belum ter-install — pasang via Ekstensi"))?;
    let mut form = file.search_form.clone();
    if !form.is_object() {
        return Ok(serde_json::Value::Null);
    }
    let defs = form
        .get("dataSources")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if defs.is_empty() {
        return Ok(form);
    }
    let mut resolved = serde_json::Map::new();
    for (name, def) in defs {
        let mut entry = def.clone();
        if let Some(ep) = def.get("endpoint").and_then(|e| e.as_str()) {
            let url = if ep.starts_with("http://") || ep.starts_with("https://") {
                ep.to_string()
            } else {
                format!("{}{}", file.base_url, ep)
            };
            match state.http.get(&url, &id).await {
                Ok(body) => match resolve_form_options(&body, &def) {
                    Some(opts) => {
                        entry["options"] = opts;
                    }
                    None => {
                        entry["options"] = serde_json::Value::Array(vec![]);
                        entry["error"] =
                            serde_json::Value::String("gagal parse respons endpoint".to_string());
                    }
                },
                Err(e) => {
                    entry["options"] = serde_json::Value::Array(vec![]);
                    entry["error"] = serde_json::Value::String(e.to_string());
                }
            }
        }
        resolved.insert(name, entry);
    }
    form["dataSources"] = serde_json::Value::Object(resolved);
    // Field `tagSourceUrl` (mobile TagDataManager.list payload
    // `[[id,name,slug,typeCode,count],...]`, saring via `tagType`).
    if let Some(params) = form.get_mut("params").and_then(|p| p.as_object_mut()) {
        for (_key, def) in params.iter_mut() {
            let url = def
                .get("tagSourceUrl")
                .and_then(|u| u.as_str())
                .unwrap_or("");
            if url.trim().is_empty() {
                continue;
            }
            let tag_type = def.get("tagType").and_then(|t| t.as_str()).unwrap_or("");
            if let Some(opts) = resolve_tag_url_options(&state.http, &id, url, tag_type).await {
                if !opts.is_empty() {
                    def["options"] = serde_json::Value::Array(opts);
                }
            }
        }
    }
    Ok(form)
}

/// Opsi tag dari URL (`tagSourceUrl`); `None` bila fetch/parse gagal
/// (opsi statis dipertahankan).
async fn resolve_tag_url_options(
    http: &crate::network::HttpClientManager,
    source_id: &str,
    url: &str,
    tag_type: &str,
) -> Option<Vec<serde_json::Value>> {
    let body = http.get(url, source_id).await.ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let arr = v.as_array()?;
    let want = tag_type.trim().to_lowercase();
    let mut out = Vec::new();
    for row in arr {
        let r = row.as_array()?;
        if r.len() < 4 {
            continue;
        }
        let name = r[1].as_str().unwrap_or("").trim();
        if name.is_empty() {
            continue;
        }
        let slug = r[2].as_str().unwrap_or("").trim();
        let code = r[3].as_i64().unwrap_or(-1);
        if !want.is_empty() && tag_code_name(code) != want {
            continue;
        }
        let value = if slug.is_empty() { name } else { slug };
        out.push(serde_json::json!({"value": value, "label": name}));
    }
    Some(out)
}

/// `TagType` mobile (tag_data_manager): 0 category, 1 artist, 2 parody,
/// 3 tag, 4 character, 5 group, 6 language, 8 genre.
fn tag_code_name(code: i64) -> String {
    match code {
        0 => "category",
        1 => "artist",
        2 => "parody",
        3 => "tag",
        4 => "character",
        5 => "group",
        6 => "language",
        8 => "genre",
        _ => "",
    }
    .to_string()
}

/// Petakan respons endpoint `dataSources` jadi `[{value,label,group?}]`.
fn resolve_form_options(body: &str, def: &serde_json::Value) -> Option<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let items_path = def
        .get("itemsPath")
        .and_then(|s| s.as_str())
        .unwrap_or("data");
    let value_path = def
        .get("valuePath")
        .and_then(|s| s.as_str())
        .unwrap_or("id");
    let label_path = def
        .get("labelPath")
        .and_then(|s| s.as_str())
        .unwrap_or(value_path);
    let group_path = def.get("groupPath").and_then(|s| s.as_str());
    let items = nav_json(&v, items_path)?.as_array()?;
    let mut out = Vec::new();
    for item in items {
        let value = nav_json(item, value_path).and_then(json_scalar)?;
        let label = nav_json(item, label_path)
            .and_then(json_scalar)
            .unwrap_or_else(|| value.clone());
        let mut opt = serde_json::Map::new();
        opt.insert("value".to_string(), serde_json::Value::String(value));
        opt.insert("label".to_string(), serde_json::Value::String(label));
        if let Some(gp) = group_path {
            if let Some(g) = nav_json(item, gp).and_then(json_scalar) {
                opt.insert("group".to_string(), serde_json::Value::String(g));
            }
        }
        out.push(serde_json::Value::Object(opt));
    }
    Some(serde_json::Value::Array(out))
}

/// Navigasi JSON bertitik (`attributes.name.en`), indeks array angka didukung.
fn nav_json<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for seg in path.split('.') {
        if seg.is_empty() || seg == "$" {
            continue;
        }
        if let Ok(idx) = seg.parse::<usize>() {
            cur = cur.as_array()?.get(idx)?;
        } else {
            cur = cur.get(seg)?;
        }
    }
    Some(cur)
}

fn json_scalar(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(n) = v.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = v.as_u64() {
        return Some(n.to_string());
    }
    if let Some(b) = v.as_bool() {
        return Some(b.to_string());
    }
    None
}

/// Query tap-tag dari `navigation.tagQueryMapping` config (bukan hardcode):
/// `raw:param=value` siap kirim ke `cmd_search`.
#[tauri::command]
pub async fn cmd_tag_query(
    state: State<'_, AppState>,
    source: String,
    tag_type: String,
    tag_name: String,
) -> Result<String, String> {
    use crate::data::datasources::config::SourceConfigs;
    let id = source.to_lowercase();
    let overlay = SourceConfigs::load_overlay(&SourceConfigs::bundled_dir(), &state.ext_dir)
        .map_err(|e| e.to_string())?;
    let file = overlay
        .get(&id)
        .ok_or_else(|| format!("sumber '{id}' belum ter-install — pasang via Ekstensi"))?;
    let mapping = file
        .navigation
        .get("tagQueryMapping")
        .and_then(|m| m.as_object());
    build_tag_query(mapping, &tag_type, &tag_name, &id)
}

/// Inti murni `cmd_tag_query` (teruji tanpa AppState).
fn build_tag_query(
    mapping: Option<&serde_json::Map<String, serde_json::Value>>,
    tag_type: &str,
    tag_name: &str,
    source_id: &str,
) -> Result<String, String> {
    let pick = |t: &str| mapping?.get(t).or_else(|| mapping?.get("default"));
    let def = pick(&tag_type.to_lowercase())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let mode = def.get("mode").and_then(|m| m.as_str()).unwrap_or("name");
    if mode == "name" || def.is_null() {
        return Ok(tag_name.to_string());
    }
    // rawParam: param + valueSource + prefix/suffix + transform + requiredPattern.
    let param = def.get("param").and_then(|p| p.as_str()).unwrap_or("");
    if param.is_empty() {
        return Ok(tag_name.to_string());
    }
    let source_kind = def
        .get("valueSource")
        .and_then(|s| s.as_str())
        .unwrap_or("tagName");
    let mut value = match source_kind {
        "tagId" => def
            .get("tagId")
            .and_then(|v| v.as_str())
            .unwrap_or(tag_name)
            .to_string(),
        "tagIdOrName" => {
            let v = def.get("tagId").and_then(|x| x.as_str()).unwrap_or("");
            if v.is_empty() {
                tag_name.to_string()
            } else {
                v.to_string()
            }
        }
        _ => tag_name.to_string(),
    };
    if def.get("transform").and_then(|t| t.as_str()) == Some("lowercase") {
        value = value.to_lowercase();
    }
    if let Some(pat) = def.get("requiredPattern").and_then(|p| p.as_str()) {
        if regex::Regex::new(pat)
            .map(|re| !re.is_match(&value))
            .unwrap_or(false)
        {
            return Err(format!("nilai tag tak valid untuk sumber '{source_id}'"));
        }
    }
    let prefix = def
        .get("valuePrefix")
        .and_then(|p| p.as_str())
        .unwrap_or("");
    let suffix = def
        .get("valueSuffix")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    Ok(format!("raw:{param}={prefix}{value}{suffix}"))
}

#[tauri::command]
pub async fn cmd_get_detail(
    state: State<'_, AppState>,
    content_id: String,
    source: Option<String>,
) -> Result<Content, String> {
    // Ikut sumber aktif seperti search/home_feed (dulu mock-only).
    let repo = match source.as_deref() {
        Some(id) if !id.is_empty() && id != "semua" => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
        _ => state.content_repo.clone(),
    };
    GetContentDetailUseCase::new(repo)
        .execute(&content_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_chapters(
    state: State<'_, AppState>,
    content_id: String,
    source: Option<String>,
    language: Option<String>,
    offset: Option<u32>,
) -> Result<Vec<Chapter>, String> {
    // Ikut sumber aktif seperti search/home_feed (dulu mock-only).
    let repo = match source.as_deref() {
        Some(id) if !id.is_empty() && id != "semua" => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
        _ => state.content_repo.clone(),
    };
    GetChaptersUseCase::new(repo)
        .execute(&content_id, language.as_deref(), offset)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_page_images(
    state: State<'_, AppState>,
    chapter_id: String,
    source: Option<String>,
) -> Result<Vec<PageImageResult>, String> {
    // Ikut sumber aktif seperti search/home_feed (dulu mock-only).
    let repo = match source.as_deref() {
        Some(id) if !id.is_empty() && id != "semua" => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
        _ => state.content_repo.clone(),
    };
    GetPageImagesUseCase::new(repo)
        .execute(&chapter_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_related(
    state: State<'_, AppState>,
    content_id: String,
    source: Option<String>,
) -> Result<Vec<Content>, String> {
    // Ikut sumber aktif seperti search/home_feed.
    let repo = match source.as_deref() {
        Some(id) if !id.is_empty() && id != "semua" => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
        _ => state.content_repo.clone(),
    };
    GetRelatedContentUseCase::new(repo)
        .execute(&content_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_get_comments(
    state: State<'_, AppState>,
    content_id: String,
    source: Option<String>,
) -> Result<Vec<Comment>, String> {
    // Ikut sumber aktif seperti search/home_feed.
    let repo = match source.as_deref() {
        Some(id) if !id.is_empty() && id != "semua" => state
            .repo_for(&id.to_lowercase())
            .map_err(|e| e.to_string())?,
        _ => state.content_repo.clone(),
    };
    GetCommentsUseCase::new(repo)
        .execute(&content_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MockContentRepository;

    #[test]
    fn new_usecases_serve_mock() {
        let detail = tauri::async_runtime::block_on(
            GetContentDetailUseCase::new(MockContentRepository).execute("m1"),
        )
        .unwrap();
        assert!(detail.title.contains("m1"));
        let found = tauri::async_runtime::block_on(
            SearchContentUseCase::new(MockContentRepository).execute(SearchFilter {
                query: "x".into(),
                source_id: None,
                page: 1,
            }),
        )
        .unwrap();
        assert_eq!(found.len(), 8);
        assert!(tauri::async_runtime::block_on(
            GetChaptersUseCase::new(MockContentRepository).execute("m1", None, None)
        )
        .unwrap()
        .is_empty());
        assert!(tauri::async_runtime::block_on(
            GetPageImagesUseCase::new(MockContentRepository).execute("c1")
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn tag_query_follows_mapping() {
        use serde_json::json;
        let mapping = json!({
            "artist": {"mode": "rawParam", "param": "f_search", "valueSource": "tagName",
                       "valuePrefix": "artist:\"", "valueSuffix": "\""},
            "default": {"mode": "rawParam", "param": "f_search", "valueSource": "tagIdOrName"}
        });
        let m = mapping.as_object();
        assert_eq!(
            build_tag_query(m, "artist", "Tappa", "ehentai").unwrap(),
            "raw:f_search=artist:\"Tappa\""
        );
        assert_eq!(
            build_tag_query(m, "group", "G", "ehentai").unwrap(),
            "raw:f_search=G"
        );
        assert_eq!(build_tag_query(None, "tag", "x", "y").unwrap(), "x");
    }

    /// Def dataSource `mangaTags` config-driven (disalin dari bentuk
    /// config installed mangadex v1.1.10 — endpoint + JSON paths).
    fn mangatags_def() -> serde_json::Value {
        serde_json::json!({
            "endpoint": "/manga/tag",
            "itemsPath": "data",
            "valuePath": "id",
            "labelPath": "attributes.name.en",
            "groupPath": "attributes.group"
        })
    }

    #[test]
    fn mangatags_def_maps_value_label_group() {
        // Murni: `resolve_form_options` atas fixture `/manga/tag` mini.
        let body = r#"{"data": [
            {"id": "tag-uuid-1", "attributes": {
                "name": {"en": "Action"}, "group": "genre"}},
            {"id": "tag-uuid-2", "attributes": {
                "name": {"en": "Gore"}, "group": "content"}}
        ]}"#;
        let opts = resolve_form_options(body, &mangatags_def()).expect("parse tag");
        let arr = opts.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("value").unwrap(), "tag-uuid-1");
        assert_eq!(arr[0].get("label").unwrap(), "Action");
        assert_eq!(arr[0].get("group").unwrap(), "genre");
        assert_eq!(arr[1].get("group").unwrap(), "content");
        // Label kosong → fallback ke value.
        let no_label = r#"{"data": [{"id": "x", "attributes": {"group": "g"}}]}"#;
        let opts = resolve_form_options(no_label, &mangatags_def()).unwrap();
        assert_eq!(opts[0].get("label").unwrap(), "x");
    }

    /// Live: `/manga/tag` sungguhan (butuh internet). Katalog tag hanya
    /// bertambah (tercatat 77 pada 2026-09-20) → batas bawah, bukan eksak.
    #[cfg(feature = "live-tests")]
    #[test]
    fn live_mangadex_mangatags_resolves() {
        let def = mangatags_def();
        let url = format!(
            "https://api.mangadex.org{}",
            def.get("endpoint").and_then(|e| e.as_str()).unwrap()
        );
        assert_eq!(url, "https://api.mangadex.org/manga/tag");
        // Sama seperti AppState produksi: `HttpClientManager::new()` (proxy on).
        let http = crate::network::HttpClientManager::new().unwrap();
        let body = tauri::async_runtime::block_on(http.get(&url, "mangadex")).unwrap();
        let opts = resolve_form_options(&body, &def).expect("parse tag live");
        let arr = opts.as_array().unwrap();
        assert!(arr.len() >= 70, "opsi mangaTags susut: {}", arr.len());
        assert!(arr[0].get("value").and_then(|v| v.as_str()).unwrap().len() > 10);
        assert!(arr[0].get("label").and_then(|v| v.as_str()).is_some());
        assert!(arr[0].get("group").and_then(|v| v.as_str()).is_some());
    }
}
