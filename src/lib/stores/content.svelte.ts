// Content store — ganti `ContentBloc/HomeBloc` (spec §14).
// Pagination: page-1 reset, loadMore append + dedupe (anti each_key_duplicate).
import { api } from "../api/client";
import type { Content } from "../domain/types";

const PAGE_SIZE_HINT = 20;

function dedupe(items: Content[], seen: Set<string>): Content[] {
  return items.filter((c) => {
    if (seen.has(c.id)) return false;
    seen.add(c.id);
    return true;
  });
}

class ContentStore {
  feed = $state<Content[]>([]);
  loading = $state(false);
  loadingMore = $state(false);
  error = $state<string | null>(null);
  page = $state(1);
  hasMore = $state(true);
  source = $state("semua");
  // Mode pencarian aktif (mobile: header "Hasil Pencarian" + bar Kueri).
  // `mode` bedakan teks-cepat vs form-filter agar tombol tak bentrok.
  searchQuery = $state<string | null>(null);
  searchLabel = $state<string | null>(null);
  searchMode = $state<"text" | "filter" | null>(null);
  searching = $state(false);

  async load(source?: string) {
    this.loading = true;
    this.error = null;
    this.source = source ?? "semua";
    this.page = 1;
    this.searchQuery = null;
    this.searchLabel = null;
    this.searchMode = null;
    try {
      const batch = await api.homeFeed(source, 1);
      this.feed = dedupe(batch, new Set());
      this.hasMore = batch.length >= PAGE_SIZE_HINT;
    } catch (e) {
      this.error = `gagal muat feed: ${e}`;
    } finally {
      this.loading = false;
    }
  }

  /** Cari umum — teks biasa = mode text, `raw:` (form) = mode filter. */
  async search(source: string, query: string, label?: string) {
    const q = query.trim();
    await this.runSearch(source, q, label ?? q, q.startsWith("raw:") ? "filter" : "text");
  }

  private async runSearch(
    source: string,
    q: string,
    label: string,
    mode: "text" | "filter",
  ) {
    if (!q) {
      await this.load(source);
      return;
    }
    this.searching = true;
    this.error = null;
    this.source = source;
    this.page = 1;
    this.searchQuery = q;
    this.searchLabel = label;
    this.searchMode = mode;
    try {
      const batch = await api.search({
        query: q,
        source_id: source === "semua" ? null : source,
        page: 1,
      });
      this.feed = dedupe(batch, new Set());
      this.hasMore = batch.length >= PAGE_SIZE_HINT;
    } catch (e) {
      this.error = `gagal mencari: ${e}`;
    } finally {
      this.searching = false;
    }
  }

  /** Bersihkan pencarian → kembali ke feed (mobile: tombol Bersihkan). */
  async clearSearch() {
    this.searchQuery = null;
    this.searchLabel = null;
    this.searchMode = null;
    await this.load(this.source);
  }

  async loadMore() {
    if (this.loadingMore || this.loading || this.searching || !this.hasMore) return;
    this.loadingMore = true;
    try {
      const next = this.page + 1;
      const batch = this.searchQuery
        ? await api.search({
            query: this.searchQuery,
            source_id: this.source === "semua" ? null : this.source,
            page: next,
          })
        : await api.homeFeed(
            this.source === "semua" ? undefined : this.source,
            next,
          );
      const seen = new Set(this.feed.map((c) => c.id));
      this.feed = [...this.feed, ...dedupe(batch, seen)];
      this.page = next;
      this.hasMore = batch.length >= PAGE_SIZE_HINT;
    } catch (e) {
      this.error = `gagal muat halaman ${this.page + 1}: ${e}`;
    } finally {
      this.loadingMore = false;
    }
  }
}

export const contentStore = new ContentStore();
