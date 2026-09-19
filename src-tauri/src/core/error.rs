//! AppError: single error type for all layers (spec §9 Core).
//! Maps to `String` at IPC boundary (`commands/*`).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum AppError {
    Validation(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(m) | Self::Internal(m) => write!(f, "{m}"),
        }
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}
