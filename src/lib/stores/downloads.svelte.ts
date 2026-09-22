// Downloads store — daftar + kontrol unduhan (8.1).
// Mode web: backend tak ada → kosong senyap (pola libraryStore).
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, WEB_OFFLINE_MSG } from "../api/client";
import { isTauriRuntime } from "../api/platform";
import type { DownloadTask } from "../domain/types";

class DownloadsStore {
  tasks = $state<DownloadTask[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  private unlisteners: UnlistenFn[] = [];
  private listening = $state(false);

  byChapter(chapterId: string): DownloadTask | undefined {
    return this.tasks.find((t) => t.chapter_id === chapterId);
  }

  /** Muat daftar + pasang listener event backend (sekali). */
  async init() {
    if (!isTauriRuntime() || this.listening) return;
    this.listening = true;
    try {
      this.unlisteners = [
        await listen<DownloadTask>("download:progress", (e) =>
          this.upsert(e.payload),
        ),
        await listen<DownloadTask>("download:completed", (e) =>
          this.upsert(e.payload),
        ),
      ];
      await this.load();
    } catch (e) {
      this.listening = false;
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal init unduhan: ${e}`;
      }
    }
  }

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.tasks = await api.downloadList();
    } catch (e) {
      if (e instanceof Error && e.message === WEB_OFFLINE_MSG) {
        this.tasks = [];
        return;
      }
      this.error = `gagal muat unduhan: ${e}`;
    } finally {
      this.loading = false;
    }
  }

  upsert(task: DownloadTask) {
    const i = this.tasks.findIndex((t) => t.chapter_id === task.chapter_id);
    if (i >= 0) this.tasks[i] = task;
    else this.tasks.push(task);
  }

  async start(chapterId: string, contentId: string, sourceId: string, total?: number) {
    this.error = null;
    try {
      const task = await api.downloadStart(chapterId, contentId, sourceId, total);
      this.upsert(task);
    } catch (e) {
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal mulai unduh: ${e}`;
      }
    }
  }

  async pause(chapterId: string) {
    this.error = null;
    try {
      const task = await api.downloadPause(chapterId);
      this.upsert(task);
    } catch (e) {
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal pause: ${e}`;
      }
    }
  }

  async resume(chapterId: string) {
    this.error = null;
    try {
      const task = await api.downloadResume(chapterId);
      this.upsert(task);
    } catch (e) {
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal lanjut: ${e}`;
      }
    }
  }

  async remove(chapterId: string) {
    this.error = null;
    const before = this.tasks;
    this.tasks = before.filter((t) => t.chapter_id !== chapterId);
    try {
      await api.downloadRemove(chapterId);
    } catch (e) {
      this.tasks = before;
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal hapus unduhan: ${e}`;
      }
    }
  }

  dispose() {
    for (const u of this.unlisteners) u();
    this.unlisteners = [];
    this.listening = false;
  }
}

export const downloadsStore = new DownloadsStore();
