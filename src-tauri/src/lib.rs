pub mod application;
pub mod cache;
pub mod commands;
pub mod core;
pub mod data;
pub mod domain;
pub mod network;

use commands::{cmd_app_info, cmd_hello_world, cmd_home_feed};
use core::{logger, AppState};
use tauri::{
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager,
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
        .manage(AppState::default())
        .setup(|app| {
            let app_menu = Submenu::with_items(
                app,
                "Kuron",
                true,
                &[
                    &PredefinedMenuItem::about(
                        app,
                        Some("Tentang Kuron"),
                        Some(AboutMetadata {
                            name: Some("Kuron".into()),
                            version: Some(env!("CARGO_PKG_VERSION").into()),
                            authors: Some(vec!["nhasix".into()]),
                            website: Some(
                                "https://github.com/shirokun20/kuron-mobile".into(),
                            ),
                            ..Default::default()
                        }),
                    )?,
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
            cmd_home_feed
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
