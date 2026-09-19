//! AppState: composition root payload, di-`manage` di lib.rs.
//! Ganti `get_it` `service_locator.dart`. Fase 0: metadata saja;
//! repos/usecases Arc<dyn> nyusul Fase 0 lengkap.

#[derive(Debug, Clone)]
pub struct AppState {
    pub app_name: String,
    pub version: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            app_name: "Kuron Desktop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
