//! Commands: thin IPC handlers (spec §15).
//! UI -> `invoke('cmd_*')` -> UseCase. No logic here.

pub mod content_commands;
pub mod hello_commands;

pub use content_commands::cmd_home_feed;
pub use hello_commands::{cmd_app_info, cmd_hello_world, AppInfo};
