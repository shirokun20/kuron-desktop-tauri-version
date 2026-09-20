//! Logger — `tracing` subscriber (Fase 0 lengkap).
//! Aturan privasi (spec §17): jangan log prompt/image,
//! glossary cuma on/off. Level via `RUST_LOG`, default `info`.

use std::sync::OnceLock;
use tracing_subscriber::{fmt, EnvFilter};

static INIT: OnceLock<()> = OnceLock::new();

pub fn init() {
    INIT.get_or_init(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = fmt().with_env_filter(filter).try_init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_idempotent_and_emits() {
        init();
        init(); // tidak panic saat dipanggil ulang (mis. antar test)
        tracing::info!(glossary = "on", "logger smoke test");
    }
}
