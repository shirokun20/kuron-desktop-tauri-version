//! RateLimiter — port `network.rateLimit` config mobile (spec §13).
//! Jeda minimum antar request per host (NHentai: 200ms).

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct RateLimiter {
    min_delay: Duration,
    last: Mutex<HashMap<String, Instant>>,
}

impl RateLimiter {
    pub fn new(min_delay_ms: u64) -> Self {
        Self {
            min_delay: Duration::from_millis(min_delay_ms),
            last: Mutex::new(HashMap::new()),
        }
    }

    pub fn none() -> Self {
        Self::new(0)
    }

    /// Tunggu hingga jeda minimum sejak request terakhir ke host ini lewat.
    pub async fn wait(&self, host: &str) {
        let sleep_for = {
            let mut last = self.last.lock().unwrap();
            let now = Instant::now();
            let wait = last
                .get(host)
                .and_then(|t| self.min_delay.checked_sub(now.duration_since(*t)))
                .unwrap_or_default();
            last.insert(host.to_string(), now + wait);
            wait
        };
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_min_delay_per_host() {
        let lim = RateLimiter::new(200);
        tauri::async_runtime::block_on(async {
            lim.wait("h.example").await;
            let start = Instant::now();
            lim.wait("h.example").await;
            assert!(start.elapsed() >= Duration::from_millis(150));
            // Host lain tak ikut antre.
            let start2 = Instant::now();
            lim.wait("lain.example").await;
            assert!(start2.elapsed() < Duration::from_millis(150));
        });
    }
}
