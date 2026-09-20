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
                    if res.status().is_server_error() && attempt + 1 < attempts.max(1) {
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
            tokio::time::sleep(std::time::Duration::from_millis(
                base_delay_ms * 2u64.pow(attempt),
            ))
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

/// Lookup DNS-over-HTTPS via Cloudflare JSON API.
/// Untuk diagnostik konektivitas dan (nanti) panen cookie CF — port awal
/// `dns_*` mobile. Butuh internet; test di-ignore agar CI offline tetap hijau.
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
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AppError::Network(format!("DoH json: {e}")))?;
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
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    /// Server HTTP lokal: hit 1 -> Set-Cookie + echo UA;
    /// hit 2+ -> echo Cookie yang diterima. Verifikasi UA + cookie jar.
    fn spawn_echo_server() -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                let n = hits_clone.fetch_add(1, Ordering::SeqCst) + 1;
                let mut buf = [0u8; 4096];
                let len = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..len]).to_string();
                let ua = req
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("user-agent:"))
                    .unwrap_or("user-agent: MISSING")
                    .to_string();
                let cookie = req
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("cookie:"))
                    .unwrap_or("cookie: MISSING")
                    .to_string();
                let body = format!("{ua}\n{cookie}\nhit={n}");
                let set_cookie = if n == 1 {
                    "Set-Cookie: kuron_test=1; Path=/\r\n"
                } else {
                    ""
                };
                let res = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
                    body.len(),
                    set_cookie,
                    body
                );
                stream.write_all(res.as_bytes()).unwrap();
            }
        });
        (format!("http://{addr}/"), hits)
    }

    #[test]
    fn get_sends_source_ua_and_keeps_cookies() {
        let (url, hits) = spawn_echo_server();
        let mgr = HttpClientManager::without_proxy().unwrap();
        let first = tauri::async_runtime::block_on(mgr.get(&url, "nhentai")).unwrap();
        assert!(first.contains("Chrome/126"), "UA sumber terkirim: {first}");
        assert!(first.contains("cookie: MISSING"), "hit 1 belum bawa cookie");
        let second = tauri::async_runtime::block_on(mgr.get(&url, "nhentai")).unwrap();
        assert!(
            second.contains("kuron_test=1"),
            "cookie jar mengirim balik: {second}"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn retry_recovers_from_500_but_not_403() {
        use std::sync::atomic::AtomicUsize;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        thread::spawn(move || {
            // hit 1: 500, hit 2: 200, hit 3: 403.
            for stream in listener.incoming().take(3) {
                let mut stream = stream.unwrap();
                let n = hits_clone.fetch_add(1, Ordering::SeqCst) + 1;
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let (status, body) = match n {
                    1 => ("500 Internal Server Error", "boom"),
                    2 => ("200 OK", "pulih"),
                    _ => ("403 Forbidden", "no"),
                };
                let res = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(res.as_bytes()).unwrap();
            }
        });
        let mgr = HttpClientManager::without_proxy().unwrap();
        let ok = tauri::async_runtime::block_on(mgr.get_with_retry(
            &format!("http://{addr}/"),
            "nhentai",
            3,
            1,
        ))
        .unwrap();
        assert_eq!(ok, "pulih");
        let err = tauri::async_runtime::block_on(mgr.get_with_retry(
            &format!("http://{addr}/"),
            "nhentai",
            3,
            1,
        ))
        .unwrap_err();
        assert!(err.to_string().contains("403"), "{err}");
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[test]
    #[ignore = "butuh internet (DoH Cloudflare); jalankan manual: cargo test resolve_doh -- --ignored"]
    fn resolve_doh_cloudflare() {
        let ips = tauri::async_runtime::block_on(resolve_doh("one.one.one.one")).unwrap();
        assert!(!ips.is_empty());
    }
}
