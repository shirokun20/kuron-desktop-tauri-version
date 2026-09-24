pub mod application;
pub mod cache;
pub mod commands;
pub mod core;
pub mod data;
pub mod domain;
pub mod network;

use commands::{
    cmd_ai_model_catalog, cmd_ai_provider_delete, cmd_ai_provider_save, cmd_ai_providers_list,
    cmd_app_info, cmd_bubble_model_status, cmd_detect_bubbles, cmd_download_list,
    cmd_download_pause, cmd_download_remove, cmd_download_resume, cmd_download_start,
    cmd_download_status, cmd_extension_install, cmd_extension_install_staged_zip,
    cmd_extension_install_zip_file, cmd_extension_manifest, cmd_extension_preview_zip_url,
    cmd_extension_uninstall, cmd_favorite_list, cmd_favorite_set, cmd_get_chapters,
    cmd_get_comments, cmd_get_detail, cmd_get_page_images, cmd_get_related, cmd_hello_world,
    cmd_history_clear, cmd_history_list, cmd_history_record, cmd_history_remove, cmd_home_feed,
    cmd_image_build_mosaic, cmd_image_chunk_webtoon, cmd_image_compress_page, cmd_library_clear,
    cmd_search, cmd_search_form, cmd_sources_list, cmd_tag_query, cmd_translate_page,
    TauriDownloadSink,
};
use core::{logger, AppState};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            // Sink event unduhan: progress/completed → semua jendela (8.1).
            {
                let state = app.state::<AppState>();
                state.download_manager.set_sink(Arc::new(TauriDownloadSink {
                    app: app.handle().clone(),
                }));
                let manager = state.download_manager.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = manager.hydrate_from_db().await {
                        tracing::warn!("hydrate downloads: {e}");
                    }
                });
            }
            let app_menu = Submenu::with_items(
                app,
                "Kuron",
                true,
                &[
                    &MenuItem::with_id(app, "about-kuron", "Tentang Kuron", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::show_all(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?;
            let file_menu = Submenu::with_items(
                app,
                "File",
                true,
                &[&PredefinedMenuItem::close_window(app, None)?],
            )?;
            let edit_menu = Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?;
            let view_menu = Submenu::with_items(
                app,
                "View",
                true,
                &[
                    &PredefinedMenuItem::fullscreen(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(
                        app,
                        "reload-page",
                        "Muat Ulang",
                        true,
                        Some("CmdOrCtrl+R"),
                    )?,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(
                        app,
                        "toggle-sidebar",
                        "Alihkan Sidebar",
                        true,
                        Some("CmdOrCtrl+B"),
                    )?,
                ],
            )?;
            let window_menu = Submenu::with_items(
                app,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, None)?,
                    &PredefinedMenuItem::maximize(app, None)?,
                ],
            )?;
            let help_menu = Submenu::with_items(
                app,
                "Help",
                true,
                &[&MenuItem::with_id(
                    app,
                    "open-repo",
                    "Repo Kuron Mobile",
                    true,
                    None::<&str>,
                )?],
            )?;
            app.set_menu(Menu::with_items(
                app,
                &[
                    &app_menu,
                    &file_menu,
                    &edit_menu,
                    &view_menu,
                    &window_menu,
                    &help_menu,
                ],
            )?)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "about-kuron" => {
                if let Some(win) = app.get_webview_window("about") {
                    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize {
                        width: 560.0,
                        height: 800.0,
                    }));
                    let _ = win.set_focus();
                } else {
                    let _ =
                        WebviewWindowBuilder::new(app, "about", WebviewUrl::App("/#about".into()))
                            .title("Tentang Kuron")
                            .inner_size(560.0, 800.0)
                            .center()
                            .resizable(false)
                            .build();
                }
            }
            "toggle-sidebar" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.emit("toggle-sidebar", ());
                }
            }
            "reload-page" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.emit("reload-page", ());
                }
            }
            "open-repo" => {
                let _ = app
                    .opener()
                    .open_url("https://github.com/shirokun20/kuron-mobile", None::<&str>);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            cmd_hello_world,
            cmd_app_info,
            cmd_home_feed,
            cmd_search,
            cmd_search_form,
            cmd_tag_query,
            cmd_get_detail,
            cmd_get_chapters,
            cmd_get_page_images,
            cmd_get_related,
            cmd_get_comments,
            cmd_sources_list,
            cmd_extension_manifest,
            cmd_extension_install,
            cmd_extension_uninstall,
            cmd_extension_install_zip_file,
            cmd_extension_preview_zip_url,
            cmd_extension_install_staged_zip,
            cmd_history_record,
            cmd_history_list,
            cmd_history_remove,
            cmd_history_clear,
            cmd_favorite_set,
            cmd_favorite_list,
            cmd_library_clear,
            cmd_ai_providers_list,
            cmd_ai_provider_save,
            cmd_ai_provider_delete,
            cmd_ai_model_catalog,
            cmd_translate_page,
            cmd_image_chunk_webtoon,
            cmd_image_build_mosaic,
            cmd_image_compress_page,
            cmd_bubble_model_status,
            cmd_detect_bubbles,
            cmd_download_start,
            cmd_download_pause,
            cmd_download_resume,
            cmd_download_list,
            cmd_download_status,
            cmd_download_remove
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
