//! DataSources — port `kuron_generic` adapters + SQLite/kv/file (Fase 2).
//! `config`: loader JSON config-driven mobile. `local`: SQLite/KV/keychain/file.
//! `remote`: engine scraper/REST + adapter situs.

pub mod config;
pub mod extension;
pub mod local;
pub mod remote;
