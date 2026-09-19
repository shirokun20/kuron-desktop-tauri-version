//! SearchFilter — port `entities/search_filter.dart` (Freezed -> serde).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilter {
    pub query: String,
    pub source_id: Option<String>,
    pub page: u32,
}
