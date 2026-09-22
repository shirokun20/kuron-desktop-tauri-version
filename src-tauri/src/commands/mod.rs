//! Commands: thin IPC handlers (spec §15).
//! UI -> `invoke('cmd_*')` -> UseCase. No logic here.

pub mod ai_commands;
pub mod content_commands;
pub mod download_commands;
pub mod extension_commands;
pub mod hello_commands;
pub mod image_commands;
pub mod library_commands;

pub use ai_commands::{
    cmd_ai_model_catalog, cmd_ai_provider_delete, cmd_ai_provider_save, cmd_ai_providers_list,
};
pub use content_commands::{
    cmd_get_chapters, cmd_get_comments, cmd_get_detail, cmd_get_page_images, cmd_get_related,
    cmd_home_feed, cmd_search, cmd_search_form, cmd_tag_query,
};
pub use download_commands::{
    cmd_download_list, cmd_download_pause, cmd_download_remove, cmd_download_resume,
    cmd_download_start, cmd_download_status, TauriDownloadSink,
};
pub use extension_commands::{
    cmd_extension_install, cmd_extension_install_staged_zip, cmd_extension_install_zip_file,
    cmd_extension_manifest, cmd_extension_preview_zip_url, cmd_extension_uninstall,
    cmd_sources_list,
};
pub use hello_commands::{cmd_app_info, cmd_hello_world, AppInfo};
pub use image_commands::{
    cmd_image_build_mosaic, cmd_image_chunk_webtoon, cmd_image_compress_page,
};
pub use library_commands::{
    cmd_favorite_list, cmd_favorite_set, cmd_history_clear, cmd_history_list, cmd_history_record,
    cmd_history_remove, cmd_library_clear,
};
