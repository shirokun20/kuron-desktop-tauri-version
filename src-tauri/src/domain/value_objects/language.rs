//! Language value object.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum Language {
    En,
    Id,
    Zh,
    Other(String),
}
