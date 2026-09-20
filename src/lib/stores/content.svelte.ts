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

  async load(source?: string) {
    this.loading = true;
    this.error = null;
    this.source = source ?? "semua";
    this.page = 1;
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

  async loadMore() {
    if (this.loadingMore || this.loading || !this.hasMore) return;
    this.loadingMore = true;
    try {
      const next = this.page + 1;
      const batch = await api.homeFeed(
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
