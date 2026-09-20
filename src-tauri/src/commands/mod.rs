//! Commands: thin IPC handlers (spec §15).
//! UI -> `invoke('cmd_*')` -> UseCase. No logic here.

pub mod content_commands;
pub mod extension_commands;
pub mod hello_commands;

pub use content_commands::{
    cmd_get_chapters, cmd_get_detail, cmd_get_page_images, cmd_home_feed, cmd_search,
    cmd_search_form, cmd_tag_query,
};
pub use extension_commands::{
    cmd_extension_install, cmd_extension_install_zip_file, cmd_extension_install_zip_url, cmd_extension_manifest,
    cmd_extension_uninstall, cmd_sources_list,
};
pub use hello_commands::{cmd_app_info, cmd_hello_world, AppInfo};
