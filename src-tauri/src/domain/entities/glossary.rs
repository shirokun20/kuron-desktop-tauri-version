//! Glossary — port `entities/glossary.dart`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct GlossaryEntry {
    pub source: String,
    pub target: String,
}
