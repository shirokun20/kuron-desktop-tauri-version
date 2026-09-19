// Content store — ganti `ContentBloc/HomeBloc` (spec §14).
import { api } from "../api/client";
import type { Content } from "../domain/types";

class ContentStore {
  feed = $state<Content[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.feed = await api.homeFeed();
    } catch (e) {
      this.error = `gagal muat feed: ${e}`;
    } finally {
      this.loading = false;
    }
  }
}

export const contentStore = new ContentStore();
