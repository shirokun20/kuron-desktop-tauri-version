//! Data-layer services (adapter infra yang bukan repository murni).

pub mod http_page_fetcher;

pub use http_page_fetcher::http_fetcher_arc;
