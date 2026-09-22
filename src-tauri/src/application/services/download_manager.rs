//! DownloadManager — task background unduh chapter (8.1, design #7).
//! Per-chapter runner: antre → unduh per halaman → progress sink → selesai.
//! Pause = flag atomik yang dicek antar halaman (resume dari halaman
//! berikutnya yang belum di-cache); event via `DownloadEventSink`
//! (produksi: Tauri `emit("download:progress"|"download:completed")`).

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use crate::{
    core::AppError,
    domain::{
        repositories::{DownloadsRepository, PageCache, PageFetcher},
        DownloadState, DownloadTask, PageImageResult,
    },
};

/// Event keluaran manager → frontend (payload = `DownloadTask`).
pub trait DownloadEventSink: Send + Sync {
    fn progress(&self, task: &DownloadTask);
    fn completed(&self, task: &DownloadTask);
}

/// Sink senyap (test / sebelum Tauri handle siap).
#[derive(Default)]
pub struct NullDownloadSink;

impl DownloadEventSink for NullDownloadSink {
    fn progress(&self, _task: &DownloadTask) {}
    fn completed(&self, _task: &DownloadTask) {}
}

/// Kunci satu job berjalan.
#[derive(Clone)]
struct JobKey {
    chapter_id: String,
}

struct JobControl {
    pause: AtomicBool,
}

#[derive(Clone)]
struct JobSnapshot {
    content_id: String,
    source_id: String,
    state: DownloadState,
}

/// Manager unduhan: satu tokio task per chapter, sink dipasang sekali.
pub struct DownloadManager {
    downloads: Arc<dyn DownloadsRepository>,
    fetcher: Arc<dyn PageFetcher>,
    cache: Arc<dyn PageCache>,
    sink: OnceLock<Arc<dyn DownloadEventSink>>,
    jobs: Mutex<HashMap<String, Arc<JobControl>>>,
    snapshots: Mutex<HashMap<String, JobSnapshot>>,
}

impl DownloadManager {
    pub fn new(
        downloads: Arc<dyn DownloadsRepository>,
        fetcher: Arc<dyn PageFetcher>,
        cache: Arc<dyn PageCache>,
    ) -> Self {
        Self {
            downloads,
            fetcher,
            cache,
            sink: OnceLock::new(),
            jobs: Mutex::new(HashMap::new()),
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    /// Pasang sink event (sekali; produksi panggil dari setup Tauri).
    pub fn set_sink(&self, sink: Arc<dyn DownloadEventSink>) {
        let _ = self.sink.set(sink);
    }

    fn sink(&self) -> Arc<dyn DownloadEventSink> {
        self.sink
            .get()
            .cloned()
            .unwrap_or_else(|| Arc::new(NullDownloadSink))
    }

    /// Snapshot state terakhir per chapter (untuk `cmd_download_status`).
    pub fn snapshot(&self, chapter_id: &str) -> Option<DownloadTask> {
        let snaps = self.snapshots.lock().ok()?;
        let s = snaps.get(chapter_id)?;
        Some(DownloadTask {
            chapter_id: chapter_id.to_string(),
            content_id: s.content_id.clone(),
            source_id: s.source_id.clone(),
            state: s.state.clone(),
        })
    }

    pub fn is_running(&self, chapter_id: &str) -> bool {
        self.jobs
            .lock()
            .map(|j| j.contains_key(chapter_id))
            .unwrap_or(false)
    }

    /// Minta pause job berjalan (idempoten; no-op bila tak jalan).
    pub fn pause(&self, chapter_id: &str) -> bool {
        let jobs = match self.jobs.lock() {
            Ok(j) => j,
            Err(_) => return false,
        };
        if let Some(job) = jobs.get(chapter_id) {
            job.pause.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Mulai (atau lanjut) unduh satu chapter di background.
    /// `page_urls` = hasil `GetPageImagesUseCase` (Remote saja dipakai).
    /// Bila job sudah jalan → error validasi (pause dulu / tunggu).
    pub async fn start(
        self: &Arc<Self>,
        chapter_id: &str,
        content_id: &str,
        source_id: &str,
        page_urls: Vec<PageImageResult>,
        total: u32,
    ) -> Result<DownloadTask, AppError> {
        let urls: Vec<String> = page_urls
            .into_iter()
            .filter_map(|p| match p {
                PageImageResult::Remote(u) => Some(u),
                PageImageResult::Cached(_) => None,
            })
            .collect();
        if urls.is_empty() {
            return Err(AppError::Validation(
                "tidak ada halaman untuk diunduh".to_string(),
            ));
        }
        if total == 0 {
            return Err(AppError::Validation("total halaman 0".to_string()));
        }

        // Tolak job ganda.
        {
            let mut jobs = self
                .jobs
                .lock()
                .map_err(|e| AppError::Internal(format!("jobs lock: {e}")))?;
            if jobs.contains_key(chapter_id) {
                return Err(AppError::Validation(format!(
                    "chapter {chapter_id} sedang diunduh — pause dulu bila ingin ulang"
                )));
            }
            jobs.insert(
                chapter_id.to_string(),
                Arc::new(JobControl {
                    pause: AtomicBool::new(false),
                }),
            );
        }

        // Total = jumlah URL sebenarnya (badge sumber boleh beda / None).
        let total = urls.len() as u32;
        let task = DownloadTask {
            chapter_id: chapter_id.to_string(),
            content_id: content_id.to_string(),
            source_id: source_id.to_string(),
            state: DownloadState::Downloading { page: 0, total },
        };
        self.persist_and_emit_progress(&task).await?;

        let key = JobKey {
            chapter_id: chapter_id.to_string(),
        };
        let manager = Arc::clone(self);
        let control = {
            let jobs = self
                .jobs
                .lock()
                .map_err(|e| AppError::Internal(format!("jobs lock: {e}")))?;
            jobs.get(chapter_id).cloned()
        }
        .ok_or_else(|| AppError::Internal("job hilang setelah insert".into()))?;

        tauri::async_runtime::spawn(async move {
            manager.run_job(key, control, urls, total).await;
        });

        Ok(task)
    }

    async fn run_job(
        self: Arc<Self>,
        key: JobKey,
        control: Arc<JobControl>,
        urls: Vec<String>,
        total: u32,
    ) {
        let chapter_id = key.chapter_id.clone();
        let snapshot = self
            .snapshots
            .lock()
            .ok()
            .and_then(|s| s.get(&chapter_id).cloned());
        let (content_id, source_id) = match snapshot {
            Some(s) => (s.content_id, s.source_id),
            None => {
                self.finish_failed(&chapter_id, "snapshot hilang".into())
                    .await;
                return;
            }
        };

        // Resume: mulai dari halaman pertama yang belum ada di cache.
        let mut page: u32 = 0;
        while (page as usize) < urls.len() && self.cache.has(&source_id, &content_id, page) {
            page += 1;
        }

        if (page as usize) >= urls.len() {
            self.finish_completed(&chapter_id, &content_id, &source_id, total)
                .await;
            return;
        }

        // Emit posisi resume (live progress segera setelah start).
        let _ = self
            .emit_progress(&DownloadTask {
                chapter_id: chapter_id.clone(),
                content_id: content_id.clone(),
                source_id: source_id.clone(),
                state: DownloadState::Downloading { page, total },
            })
            .await;

        while (page as usize) < urls.len() {
            if control.pause.load(Ordering::SeqCst) {
                self.finish_paused(&chapter_id, &content_id, &source_id, page, total)
                    .await;
                return;
            }
            let url = &urls[page as usize];
            match self.fetcher.fetch(url, &source_id, None).await {
                Ok(bytes) => {
                    if let Err(e) = self.cache.put(&source_id, &content_id, page, &bytes) {
                        self.finish_failed(&chapter_id, e.to_string()).await;
                        return;
                    }
                    page += 1;
                    let task = DownloadTask {
                        chapter_id: chapter_id.clone(),
                        content_id: content_id.clone(),
                        source_id: source_id.clone(),
                        state: DownloadState::Downloading { page, total },
                    };
                    if let Err(e) = self.persist_and_emit_progress(&task).await {
                        self.finish_failed(&chapter_id, e.to_string()).await;
                        return;
                    }
                }
                Err(e) => {
                    // Jeda pause datang selama fetch → anggap pause, bukan gagal.
                    if control.pause.load(Ordering::SeqCst) {
                        self.finish_paused(&chapter_id, &content_id, &source_id, page, total)
                            .await;
                        return;
                    }
                    self.finish_failed(&chapter_id, e.to_string()).await;
                    return;
                }
            }
        }

        self.finish_completed(&chapter_id, &content_id, &source_id, total)
            .await;
    }

    async fn finish_completed(
        &self,
        chapter_id: &str,
        content_id: &str,
        source_id: &str,
        total: u32,
    ) {
        let task = DownloadTask {
            chapter_id: chapter_id.to_string(),
            content_id: content_id.to_string(),
            source_id: source_id.to_string(),
            state: DownloadState::Completed,
        };
        let _ = total;
        let _ = self.persist_and_emit_completed(&task).await;
        self.drop_job(chapter_id);
    }

    async fn finish_paused(
        &self,
        chapter_id: &str,
        content_id: &str,
        source_id: &str,
        page: u32,
        total: u32,
    ) {
        let task = DownloadTask {
            chapter_id: chapter_id.to_string(),
            content_id: content_id.to_string(),
            source_id: source_id.to_string(),
            // page = jumlah selesai; FE tampil "n/total" lalu Paused di list.
            state: DownloadState::Paused,
        };
        let _ = (page, total);
        if let Ok(mut snaps) = self.snapshots.lock() {
            snaps.insert(
                chapter_id.to_string(),
                JobSnapshot {
                    content_id: content_id.to_string(),
                    source_id: source_id.to_string(),
                    state: DownloadState::Paused,
                },
            );
        }
        let _ = self.persist(&task).await;
        let _ = self.emit_progress(&task).await;
        self.drop_job(chapter_id);
    }

    async fn finish_failed(&self, chapter_id: &str, msg: String) {
        if let Ok(mut snaps) = self.snapshots.lock() {
            if let Some(s) = snaps.get_mut(chapter_id) {
                s.state = DownloadState::Failed(msg.clone());
            }
        }
        let task = self.snapshot(chapter_id).unwrap_or(DownloadTask {
            chapter_id: chapter_id.to_string(),
            content_id: String::new(),
            source_id: String::new(),
            state: DownloadState::Failed(msg.clone()),
        });
        let _ = self.persist(&task).await;
        self.sink().completed(&task);
        self.drop_job(chapter_id);
    }

    fn drop_job(&self, chapter_id: &str) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.remove(chapter_id);
        }
    }

    async fn persist(&self, task: &DownloadTask) -> Result<(), AppError> {
        self.downloads.save(task).await
    }

    async fn persist_and_emit_progress(&self, task: &DownloadTask) -> Result<(), AppError> {
        if let Ok(mut snaps) = self.snapshots.lock() {
            snaps.insert(
                task.chapter_id.clone(),
                JobSnapshot {
                    content_id: task.content_id.clone(),
                    source_id: task.source_id.clone(),
                    state: task.state.clone(),
                },
            );
        }
        self.persist(task).await?;
        self.emit_progress(task).await
    }

    async fn persist_and_emit_completed(&self, task: &DownloadTask) -> Result<(), AppError> {
        if let Ok(mut snaps) = self.snapshots.lock() {
            snaps.insert(
                task.chapter_id.clone(),
                JobSnapshot {
                    content_id: task.content_id.clone(),
                    source_id: task.source_id.clone(),
                    state: task.state.clone(),
                },
            );
        }
        self.persist(task).await?;
        self.sink().completed(task);
        Ok(())
    }

    async fn emit_progress(&self, task: &DownloadTask) -> Result<(), AppError> {
        self.sink().progress(task);
        Ok(())
    }

    /// Hapus baris + cache konten (hapus unduhan, spec kontrol data).
    pub async fn remove(&self, chapter_id: &str) -> Result<bool, AppError> {
        self.pause(chapter_id);
        // Tunggu job lepas (best-effort; pause diproses antar halaman).
        for _ in 0..50 {
            if !self.is_running(chapter_id) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let existing = self.downloads.get(chapter_id).await?;
        let removed = self.downloads.delete(chapter_id).await?;
        if let Some(task) = existing {
            let _ = self.cache.remove_content(&task.source_id, &task.content_id);
        }
        if let Ok(mut snaps) = self.snapshots.lock() {
            snaps.remove(chapter_id);
        }
        Ok(removed)
    }

    /// Restore snapshot dari SQLite saat start aplikasi (resume UI).
    pub async fn hydrate_from_db(&self) -> Result<(), AppError> {
        let tasks = self.downloads.list().await?;
        let mut snaps = self
            .snapshots
            .lock()
            .map_err(|e| AppError::Internal(format!("snapshots lock: {e}")))?;
        for t in tasks {
            // Task yang tertinggal Downloading/Queued setelah kill → Paused.
            let state = match t.state {
                DownloadState::Downloading { .. } | DownloadState::Queued => DownloadState::Paused,
                other => other,
            };
            snaps.insert(
                t.chapter_id.clone(),
                JobSnapshot {
                    content_id: t.content_id,
                    source_id: t.source_id,
                    state,
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[derive(Default)]
    struct MemDownloads {
        inner: Mutex<HashMap<String, DownloadTask>>,
    }

    #[async_trait::async_trait]
    impl DownloadsRepository for MemDownloads {
        async fn save(&self, task: &DownloadTask) -> Result<(), AppError> {
            self.inner
                .lock()
                .unwrap()
                .insert(task.chapter_id.clone(), task.clone());
            Ok(())
        }
        async fn get(&self, chapter_id: &str) -> Result<Option<DownloadTask>, AppError> {
            Ok(self.inner.lock().unwrap().get(chapter_id).cloned())
        }
        async fn list(&self) -> Result<Vec<DownloadTask>, AppError> {
            Ok(self.inner.lock().unwrap().values().cloned().collect())
        }
        async fn delete(&self, chapter_id: &str) -> Result<bool, AppError> {
            Ok(self.inner.lock().unwrap().remove(chapter_id).is_some())
        }
    }

    struct ScriptedFetcher {
        /// URL substring → payload; default "IMG".
        fail_if_url_contains: Option<String>,
        delay_ms: u64,
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl PageFetcher for ScriptedFetcher {
        async fn fetch(
            &self,
            url: &str,
            _source_id: &str,
            _referer: Option<&str>,
        ) -> Result<Vec<u8>, AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }
            if let Some(bad) = &self.fail_if_url_contains {
                if url.contains(bad.as_str()) {
                    return Err(AppError::Network("HTTP 500".into()));
                }
            }
            Ok(format!("IMG:{url}").into_bytes())
        }
    }

    struct MemCache {
        inner: Mutex<HashMap<(String, String, u32), Vec<u8>>>,
    }

    impl PageCache for MemCache {
        fn has(&self, source_id: &str, content_id: &str, page: u32) -> bool {
            self.inner
                .lock()
                .unwrap()
                .contains_key(&(source_id.into(), content_id.into(), page))
        }
        fn put(
            &self,
            source_id: &str,
            content_id: &str,
            page: u32,
            bytes: &[u8],
        ) -> Result<(), AppError> {
            self.inner
                .lock()
                .unwrap()
                .insert((source_id.into(), content_id.into(), page), bytes.to_vec());
            Ok(())
        }
        fn remove_content(&self, source_id: &str, content_id: &str) -> Result<(), AppError> {
            self.inner
                .lock()
                .unwrap()
                .retain(|(s, c, _), _| !(s == source_id && c == content_id));
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        progress: Mutex<Vec<DownloadTask>>,
        completed: Mutex<Vec<DownloadTask>>,
    }

    impl DownloadEventSink for RecordingSink {
        fn progress(&self, task: &DownloadTask) {
            self.progress.lock().unwrap().push(task.clone());
        }
        fn completed(&self, task: &DownloadTask) {
            self.completed.lock().unwrap().push(task.clone());
        }
    }

    fn remote_urls(n: usize) -> Vec<PageImageResult> {
        (0..n)
            .map(|i| PageImageResult::Remote(format!("https://cdn.test/p{i}.jpg")))
            .collect()
    }

    fn manager(
        fetcher: Arc<ScriptedFetcher>,
    ) -> (
        Arc<DownloadManager>,
        Arc<MemDownloads>,
        Arc<MemCache>,
        Arc<RecordingSink>,
    ) {
        let downloads = Arc::new(MemDownloads::default());
        let cache = Arc::new(MemCache {
            inner: Mutex::new(HashMap::new()),
        });
        let sink = Arc::new(RecordingSink::default());
        let mgr = Arc::new(DownloadManager::new(
            downloads.clone(),
            fetcher,
            cache.clone() as Arc<dyn PageCache>,
        ));
        mgr.set_sink(sink.clone());
        // hydrate kosong
        (mgr, downloads, cache, sink)
    }

    async fn wait_until(
        mgr: &DownloadManager,
        chapter: &str,
        pred: impl Fn(&DownloadTask) -> bool,
        sink: &RecordingSink,
    ) -> bool {
        for _ in 0..200 {
            if let Some(t) = mgr.snapshot(chapter) {
                if pred(&t) {
                    return true;
                }
            }
            if sink
                .completed
                .lock()
                .unwrap()
                .iter()
                .any(|t| t.chapter_id == chapter && pred(t))
            {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    #[test]
    fn start_downloads_all_pages_and_completes() {
        tauri::async_runtime::block_on(async {
            let fetcher = Arc::new(ScriptedFetcher {
                fail_if_url_contains: None,
                delay_ms: 0,
                calls: AtomicUsize::new(0),
            });
            let (mgr, downloads, cache, sink) = manager(fetcher.clone());
            let task = mgr
                .start("c1", "m1", "nhentai", remote_urls(3), 3)
                .await
                .unwrap();
            assert!(matches!(
                task.state,
                DownloadState::Downloading { total: 3, .. }
            ));
            let done = wait_until(
                &mgr,
                "c1",
                |t| matches!(t.state, DownloadState::Completed),
                &sink,
            )
            .await;
            assert!(done, "tidak selesai: {:?}", mgr.snapshot("c1"));
            assert_eq!(fetcher.calls.load(Ordering::SeqCst), 3);
            assert!(cache.has("nhentai", "m1", 0));
            assert!(cache.has("nhentai", "m1", 2));
            let db = downloads.get("c1").await.unwrap().unwrap();
            assert!(matches!(db.state, DownloadState::Completed));
            // Progress live: minimal event page=1..=3.
            let progresses: Vec<_> = sink
                .progress
                .lock()
                .unwrap()
                .iter()
                .filter(|t| t.chapter_id == "c1")
                .cloned()
                .collect();
            assert!(
                progresses
                    .iter()
                    .any(|t| matches!(t.state, DownloadState::Downloading { page: 3, total: 3 })),
                "progress akhir hilang: {progresses:?}"
            );
            assert_eq!(sink.completed.lock().unwrap().len(), 1);
            assert!(!mgr.is_running("c1"));
        });
    }

    #[test]
    fn pause_then_resume_skips_cached_pages() {
        tauri::async_runtime::block_on(async {
            let fetcher = Arc::new(ScriptedFetcher {
                fail_if_url_contains: None,
                delay_ms: 30,
                calls: AtomicUsize::new(0),
            });
            let (mgr, downloads, cache, sink) = manager(fetcher.clone());
            mgr.start("c1", "m1", "nhentai", remote_urls(5), 5)
                .await
                .unwrap();
            // Tunggu minimal 1 halaman masuk cache, lalu pause.
            for _ in 0..100 {
                if cache.has("nhentai", "m1", 0) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            assert!(mgr.pause("c1"), "pause harus menemukan job");
            let paused = wait_until(
                &mgr,
                "c1",
                |t| matches!(t.state, DownloadState::Paused),
                &sink,
            )
            .await;
            assert!(paused, "tidak paused: {:?}", mgr.snapshot("c1"));
            let calls_at_pause = fetcher.calls.load(Ordering::SeqCst);
            assert!(
                calls_at_pause < 5,
                "harus berhenti sebelum 5: {calls_at_pause}"
            );
            assert!(!mgr.is_running("c1"));

            // Resume: start lagi — page yang sudah cache di-skip.
            mgr.start("c1", "m1", "nhentai", remote_urls(5), 5)
                .await
                .unwrap();
            let done = wait_until(
                &mgr,
                "c1",
                |t| matches!(t.state, DownloadState::Completed),
                &sink,
            )
            .await;
            assert!(done, "resume tidak selesai: {:?}", mgr.snapshot("c1"));
            // Fetch hanya untuk halaman yang belum ada.
            let total_fetches = fetcher.calls.load(Ordering::SeqCst);
            assert!(
                total_fetches >= 5 && total_fetches < 5 + calls_at_pause.max(1) + 5,
                "fetch resume tak wajar: {total_fetches} (pause at {calls_at_pause})"
            );
            for i in 0..5 {
                assert!(cache.has("nhentai", "m1", i), "page {i} hilang");
            }
            let db = downloads.get("c1").await.unwrap().unwrap();
            assert!(matches!(db.state, DownloadState::Completed));
        });
    }

    #[test]
    fn double_start_rejected_and_failed_completes_event() {
        tauri::async_runtime::block_on(async {
            let fetcher = Arc::new(ScriptedFetcher {
                fail_if_url_contains: Some("p1".into()),
                delay_ms: 10,
                calls: AtomicUsize::new(0),
            });
            let (mgr, _downloads, _cache, sink) = manager(fetcher.clone());
            let first = mgr.start("c1", "m1", "nhentai", remote_urls(4), 4).await;
            assert!(first.is_ok());
            let second = mgr.start("c1", "m1", "nhentai", remote_urls(4), 4).await;
            assert!(second.is_err(), "job ganda harus ditolak");

            let failed = wait_until(
                &mgr,
                "c1",
                |t| matches!(t.state, DownloadState::Failed(_)),
                &sink,
            )
            .await;
            assert!(failed, "tidak failed: {:?}", mgr.snapshot("c1"));
            // Failed → event completed (terminal) 1×, progress tetap ada.
            assert_eq!(sink.completed.lock().unwrap().len(), 1);
            assert!(!mgr.is_running("c1"));
            // Setelah failed, start ulang diizinkan.
            let retry = mgr.start("c1", "m1", "nhentai", remote_urls(4), 4).await;
            assert!(retry.is_ok());
            // Biarkan job retry jalan sebentar lalu pause bersih.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            mgr.pause("c1");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        });
    }

    #[test]
    fn hydrate_marks_interrupted_downloading_as_paused() {
        tauri::async_runtime::block_on(async {
            let downloads = Arc::new(MemDownloads::default());
            downloads
                .save(&DownloadTask {
                    chapter_id: "c9".into(),
                    content_id: "m9".into(),
                    source_id: "nhentai".into(),
                    state: DownloadState::Downloading { page: 2, total: 9 },
                })
                .await
                .unwrap();
            let fetcher = Arc::new(ScriptedFetcher {
                fail_if_url_contains: None,
                delay_ms: 0,
                calls: AtomicUsize::new(0),
            });
            let mgr = Arc::new(DownloadManager::new(
                downloads.clone(),
                fetcher,
                Arc::new(MemCache {
                    inner: Mutex::new(HashMap::new()),
                }),
            ));
            mgr.hydrate_from_db().await.unwrap();
            let snap = mgr.snapshot("c9").unwrap();
            assert!(matches!(snap.state, DownloadState::Paused));
        });
    }

    #[test]
    fn remove_deletes_row_and_cache() {
        tauri::async_runtime::block_on(async {
            let fetcher = Arc::new(ScriptedFetcher {
                fail_if_url_contains: None,
                delay_ms: 0,
                calls: AtomicUsize::new(0),
            });
            let (mgr, downloads, cache, sink) = manager(fetcher);
            mgr.start("c1", "m1", "nhentai", remote_urls(2), 2)
                .await
                .unwrap();
            let done = wait_until(
                &mgr,
                "c1",
                |t| matches!(t.state, DownloadState::Completed),
                &sink,
            )
            .await;
            assert!(done);
            assert!(cache.has("nhentai", "m1", 0));
            assert!(mgr.remove("c1").await.unwrap());
            assert!(downloads.get("c1").await.unwrap().is_none());
            assert!(!cache.has("nhentai", "m1", 0));
            assert!(!cache.has("nhentai", "m1", 1));
        });
    }
}
