//! Network — ganti dio+native_dio_adapter (Fase 2, spec §12).
//! `HttpClientManager`: reqwest (rustls, cookie jar, timeout 30s) + UA per sumber.
//! `dns_resolver`: lookup DoH eksplisit (Cloudflare) untuk diagnostik / CF-bypass nanti.

use std::{net::IpAddr, time::Duration};

use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Client,
};

use crate::core::AppError;

/// Desktop Chrome UA default (fallback bila sumber tak dikenal).
pub const DESKTOP_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// UA per sumber — port `kuron_user_agent.dart`.
pub fn user_agent_for(source_id: &str) -> &'static str {
    match source_id {
        "nhentai" | "hitomi" | "ehentai" | "mangadex" => DESKTOP_UA,
        _ => DESKTOP_UA,
    }
}

/// Header default untuk satu sumber (UA + accept generik).
pub fn headers_for_source(source_id: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(user_agent_for(source_id)),
    );
    headers.insert(
        "Accept",
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/json;q=0.9,*/*;q=0.8",
        ),
    );
    headers
}

pub struct HttpClientManager {
    client: Client,
}

impl Clone for HttpClientManager {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl HttpClientManager {
    pub fn new() -> Result<Self, AppError> {
        Self::build(false)
    }

    /// Tanpa system proxy — untuk test lokal (hindari env proxy makan localhost).
    pub fn without_proxy() -> Result<Self, AppError> {
        Self::build(true)
    }

    fn build(no_proxy: bool) -> Result<Self, AppError> {
        let mut builder = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .default_headers(headers_for_source("default"));
        if no_proxy {
            builder = builder.no_proxy();
        }
        Ok(Self {
            client: builder.build()?,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// GET teks (HTML/JSON) dengan header sumber.
    pub async fn get(&self, url: &str, source_id: &str) -> Result<String, AppError> {
        let text = self
            .client
            .get(url)
            .headers(headers_for_source(source_id))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }

    /// GET teks dengan retry (5xx / error transport saja; 4xx langsung gagal).
    /// Port `network.retry` config mobile (default 3x, backoff 1s → 2s → 4s).
    pub async fn get_with_retry(
        &self,
        url: &str,
        source_id: &str,
        attempts: u32,
        base_delay_ms: u64,
    ) -> Result<String, AppError> {
        let mut last_err = AppError::Network("tanpa percobaan".to_string());
        for attempt in 0..attempts.max(1) {
            match self
                .client
                .get(url)
                .headers(headers_for_source(source_id))
                .send()
                .await
            {
                Ok(res) => {
                    if should_retry_status(res.status()) && attempt + 1 < attempts.max(1) {
                        last_err = AppError::Network(format!("HTTP {}", res.status()));
                    } else {
                        return res
                            .error_for_status()
                            .map_err(AppError::from)?
                            .text()
                            .await
                            .map_err(AppError::from);
                    }
                }
                Err(e) => {
                    last_err = AppError::Network(e.to_string());
                    if e.status().map(|s| s.is_client_error()).unwrap_or(false) {
                        return Err(last_err);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(retry_backoff_ms(
                base_delay_ms,
                attempt,
            )))
            .await;
        }
        Err(last_err)
    }

    /// GET bytes (gambar) dengan header sumber + referer opsional.
    pub async fn get_bytes(
        &self,
        url: &str,
        source_id: &str,
        referer: Option<&str>,
    ) -> Result<Vec<u8>, AppError> {
        let mut req = self.client.get(url).headers(headers_for_source(source_id));
        if let Some(r) = referer {
            req = req.header("Referer", r);
        }
        let bytes = req.send().await?.error_for_status()?.bytes().await?;
        Ok(bytes.to_vec())
    }
}

/// Kebijakan retry per status HTTP (murni, tanpa network):
/// 5xx boleh coba lagi, 2xx/4xx final. Dipakai `get_with_retry`.
pub fn should_retry_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error()
}

/// Jadwal backoff eksponensial (murni): `base × 2^attempt`.
/// Port `network.retry` config mobile (1s → 2s → 4s).
pub fn retry_backoff_ms(base_delay_ms: u64, attempt: u32) -> u64 {
    base_delay_ms.saturating_mul(2u64.pow(attempt))
}

/// Lookup DNS-over-HTTPS via Cloudflare JSON API.
/// Untuk diagnostik konektivitas dan (nanti) panen cookie CF — port awal
/// `dns_*` mobile. Butuh internet; test live di feature `live-tests`.
pub async fn resolve_doh(host: &str) -> Result<Vec<IpAddr>, AppError> {
    let mgr = HttpClientManager::new()?;
    let url = format!("https://cloudflare-dns.com/dns-query?name={host}&type=A");
    let body = mgr
        .client
        .get(&url)
        .header("Accept", "application/dns-json")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    parse_doh_ips(host, &body)
}

/// Parse jawaban DoH JSON Cloudflare → IP tipe A (murni, tanpa network).
pub fn parse_doh_ips(host: &str, body: &str) -> Result<Vec<IpAddr>, AppError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AppError::Network(format!("DoH json: {e}")))?;
    let mut ips = Vec::new();
    if let Some(arr) = v.get("Answer").and_then(|a| a.as_array()) {
        for ans in arr {
            if ans.get("type").and_then(|t| t.as_u64()) == Some(1) {
                if let Some(ip) = ans
                    .get("data")
                    .and_then(|d| d.as_str())
                    .and_then(|s| s.parse::<IpAddr>().ok())
                {
                    ips.push(ip);
                }
            }
        }
    }
    if ips.is_empty() {
        return Err(AppError::Network(format!(
            "DoH: tanpa jawaban A untuk {host}"
        )));
    }
    Ok(ips)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Header per sumber murni (tanpa socket): UA Chrome desktop + Accept.
    /// Cookie jar = `cookie_store(true)` reqwest tanpa logika kustom —
    /// roundtrip HTTP-nya milik suite live (`live-tests`), bukan unit.
    #[test]
    fn headers_carry_desktop_ua_per_source() {
        use reqwest::header::USER_AGENT;
        for source in ["nhentai", "hitomi", "ehentai", "mangadex", "takdikenal"] {
            let h = headers_for_source(source);
            let ua = h.get(USER_AGENT).unwrap().to_str().unwrap();
            assert!(ua.contains("Chrome/126"), "UA {source}: {ua}");
            assert!(h.contains_key("Accept"), "Accept {source}");
        }
        // Manager ter-bangun dua mode (proxy sistem / tanpa proxy).
        assert!(HttpClientManager::new().is_ok());
        assert!(HttpClientManager::without_proxy().is_ok());
    }

    /// Kebijakan retry murni: 5xx coba lagi, 2xx/4xx final.
    #[test]
    fn retry_policy_retries_server_errors_only() {
        use reqwest::StatusCode;
        assert!(should_retry_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(should_retry_status(StatusCode::BAD_GATEWAY));
        assert!(!should_retry_status(StatusCode::OK));
        assert!(!should_retry_status(StatusCode::FORBIDDEN));
        assert!(!should_retry_status(StatusCode::NOT_FOUND));
    }

    /// Backoff eksponensial ala config mobile: base × 2^attempt.
    #[test]
    fn retry_backoff_doubles_per_attempt() {
        assert_eq!(
            (0..3).map(|a| retry_backoff_ms(1000, a)).collect::<Vec<_>>(),
            vec![1000, 2000, 4000]
        );
        assert_eq!(retry_backoff_ms(1, 0), 1);
    }

    /// Parse jawaban DoH Cloudflare murni: ambil A, lewati AAAA/invalid.
    #[test]
    fn doh_answer_parses_a_records_only() {
        let body = r#"{"Status": 0, "Answer": [
            {"name": "one.one.one.one.", "type": 1, "data": "1.1.1.1"},
            {"name": "one.one.one.one.", "type": 28, "data": "2606:4700:4700::1111"},
            {"name": "one.one.one.one.", "type": 1, "data": "bukan-ip"}
        ]}"#;
        let ips = parse_doh_ips("one.one.one.one", body).unwrap();
        assert_eq!(ips, vec!["1.1.1.1".parse::<IpAddr>().unwrap()]);
        assert!(parse_doh_ips("x", r#"{"Status": 0}"#).is_err());
        assert!(parse_doh_ips("x", "bukan json").is_err());
    }

    /// Live: DoH sungguhan ke Cloudflare (butuh internet).
    #[cfg(feature = "live-tests")]
    #[test]
    fn live_resolve_doh_cloudflare() {
        let ips = tauri::async_runtime::block_on(resolve_doh("one.one.one.one")).unwrap();
        assert!(!ips.is_empty());
    }
}
