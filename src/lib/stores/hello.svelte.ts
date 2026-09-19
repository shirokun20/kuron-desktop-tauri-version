// Hello store — Svelte 5 runes ganti `Cubit` (spec §14).
import { api } from "../api/client";
import type { AppInfo, Hello } from "../domain/types";

class HelloStore {
  hello = $state<Hello | null>(null);
  info = $state<AppInfo | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);

  async sayHello(name: string) {
    this.loading = true;
    this.error = null;
    try {
      this.hello = await api.helloWorld(name.trim() || undefined);
    } catch (e) {
      this.error = `gagal: ${e}`;
    } finally {
      this.loading = false;
    }
  }

  async loadInfo() {
    try {
      this.info = await api.appInfo();
    } catch (e) {
      this.error = `backend belum jalan (buka via browser?): ${e}`;
    }
  }
}

export const helloStore = new HelloStore();
