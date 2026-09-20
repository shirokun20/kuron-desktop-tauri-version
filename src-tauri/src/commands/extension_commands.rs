//! Extension commands: repositori sumber installable (4.7).
//! Daftar dinamis UI + install/uninstall via manifest-link/zip.

use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::{
    core::{AppState, InstalledSource},
    data::datasources::extension::{
        ExtensionManager, ExtensionManifest, ZipPreview, DEFAULT_MANIFEST_URL,
    },
};

#[tauri::command]
pub fn cmd_sources_list(state: State<'_, AppState>) -> Result<Vec<InstalledSource>, String> {
    state.source_entries().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_extension_manifest(
    state: State<'_, AppState>,
    url: Option<String>,
) -> Result<ExtensionManifest, String> {
    let url = url.unwrap_or_else(|| DEFAULT_MANIFEST_URL.to_string());
    ExtensionManifest::fetch(&state.http, &url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_extension_install(
    state: State<'_, AppState>,
    manifest_url: String,
    id: String,
) -> Result<InstalledSource, String> {
    let manifest =
        ExtensionManifest::fetch(&state.http, &manifest_url).map_err(|e| e.to_string())?;
    let entry = manifest
        .installable_sources
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("'{id}' tak ada di manifest"))?;
    let mgr = ExtensionManager::new(&state.http, &state.ext_dir).map_err(|e| e.to_string())?;
    let cfg = mgr
        .install(&manifest_url, entry)
        .map_err(|e| e.to_string())?;
    Ok(InstalledSource {
        id: entry.id.clone(),
        version: cfg.version.clone(),
        base_url: cfg.base_url.clone(),
        installed: true,
        icon_url: entry
            .meta
            .as_ref()
            .and_then(|m| m.icon_url.clone())
            .map(|u| {
                crate::data::datasources::extension::ExtensionManifest::resolve_url(
                    &manifest_url,
                    &u,
                )
            })
            // Meta manifest kosong → pakai ikon dari config sumber itu sendiri
            // (alur sama dengan sumber bundled di `source_entries`).
            .or_else(|| cfg.ui_icon_path()),
        display_name: cfg.ui_display_name(),
    })
}

#[tauri::command]
pub fn cmd_extension_uninstall(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    if id == "nhentai" {
        return Err("nhentai bawaan aplikasi, tak bisa di-uninstall".to_string());
    }
    let mgr = ExtensionManager::new(&state.http, &state.ext_dir).map_err(|e| e.to_string())?;
    mgr.uninstall(&id).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn cmd_extension_install_zip_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ZipPreview, String> {
    // WAJIB non-blocking: `blocking_pick_file` macetkan worker async Tauri
    // → spinner loading abadi (bug 2026-09-20). Callback + oneshot.
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Arsip ZIP ekstensi", &["zip"])
        .pick_file(move |picked| {
            let _ = tx.send(picked);
        });
    let picked = rx.await.map_err(|e| format!("dialog gagal: {e}"))?;
    let path = picked
        .ok_or_else(|| "pemilihan file dibatalkan".to_string())?
        .as_path()
        .ok_or_else(|| "path file tak valid".to_string())?
        .to_path_buf();
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mgr = ExtensionManager::new(&state.http, &state.ext_dir).map_err(|e| e.to_string())?;
    mgr.preview_zip_bytes(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_extension_preview_zip_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<ZipPreview, String> {
    let mgr = ExtensionManager::new(&state.http, &state.ext_dir).map_err(|e| e.to_string())?;
    let bytes = tauri::async_runtime::block_on(state.http.get_bytes(&url, "extensions", None))
        .map_err(|e| e.to_string())?;
    mgr.preview_zip_bytes(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_extension_install_staged_zip(
    state: State<'_, AppState>,
    token: String,
    selected: Vec<String>,
) -> Result<Vec<String>, String> {
    let mgr = ExtensionManager::new(&state.http, &state.ext_dir).map_err(|e| e.to_string())?;
    mgr.install_staged_zip(&token, &selected)
        .map_err(|e| e.to_string())
}
