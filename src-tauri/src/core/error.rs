//! AppError: single error type for all layers (spec §9 Core).
//! Maps to `String` at IPC boundary (`commands/*`).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum AppError {
    Validation(String),
    Network(String),
    Storage(String),
    Internal(String),
    /// 429 provider AI — UI cooldown + tawarkan fallback (7.2).
    RateLimited(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(m)
            | Self::Network(m)
            | Self::Storage(m)
            | Self::Internal(m)
            | Self::RateLimited(m) => {
                write!(f, "{m}")
            }
        }
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Storage(e.to_string())
    }
}
