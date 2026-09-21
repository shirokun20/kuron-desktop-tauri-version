// Library store — riwayat + favorit SQLite backend (8.3).
// Mode web: backend tak ada → kosong senyap (pola contentStore.offline).
import { api, WEB_OFFLINE_MSG } from "../api/client";
import type { Content, HistoryEntry } from "../domain/types";

class LibraryStore {
  favorites = $state<Content[]>([]);
  history = $state<HistoryEntry[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  private favSet(): Set<string> {
    return new Set(this.favorites.map((c) => c.id));
  }

  isFav(id: string): boolean {
    return this.favSet().has(id);
  }

  /** Muat favorit + riwayat dari backend; gagal → error (web → kosong). */
  async load() {
    this.loading = true;
    this.error = null;
    try {
      const [favs, hist] = await Promise.all([
        api.favoriteList(),
        api.historyList(50),
      ]);
      this.favorites = favs;
      this.history = hist;
    } catch (e) {
      if (e instanceof Error && e.message === WEB_OFFLINE_MSG) {
        this.favorites = [];
        this.history = [];
        return;
      }
      this.error = `gagal muat library: ${e}`;
    } finally {
      this.loading = false;
    }
  }

  /** Toggle favorit optimistik; gagal → rollback + error. */
  async toggleFav(content: Content) {
    const wasFav = this.isFav(content.id);
    this.error = null;
    this.favorites = wasFav
      ? this.favorites.filter((c) => c.id !== content.id)
      : [...this.favorites, { ...content, is_favorite: true }];
    try {
      await api.favoriteSet(content, !wasFav);
    } catch (e) {
      this.favorites = wasFav
        ? [...this.favorites, { ...content, is_favorite: true }]
        : this.favorites.filter((c) => c.id !== content.id);
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal simpan favorit: ${e}`;
      }
    }
  }

  /** Catat posisi baca (dipakai reader/detail Fase 5; API siap kini). */
  async recordHistory(contentId: string, position: number) {
    try {
      await api.historyRecord(contentId, position);
    } catch (e) {
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal catat riwayat: ${e}`;
      }
    }
  }

  async clearHistory() {
    this.error = null;
    try {
      await api.historyClear();
      this.history = [];
    } catch (e) {
      this.error = `gagal hapus riwayat: ${e}`;
    }
  }

  async clearLibrary() {
    this.error = null;
    try {
      await api.libraryClear();
      this.favorites = [];
      this.history = [];
    } catch (e) {
      this.error = `gagal reset library: ${e}`;
    }
  }
}

export const libraryStore = new LibraryStore();
