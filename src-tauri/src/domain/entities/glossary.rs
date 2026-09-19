//! Glossary — port `entities/glossary.dart`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub source: String,
    pub target: String,
}
