//! Language value object.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    En,
    Id,
    Zh,
    Other(String),
}
