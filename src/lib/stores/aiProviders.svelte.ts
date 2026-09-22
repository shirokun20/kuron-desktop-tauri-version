// AI provider store — daftar BYOK + status kunci (9.2, spec settings-ai).
// Kunci API TIDAK PERNAH disimpan di frontend — hanya `has_key` dari backend.
import { api, WEB_OFFLINE_MSG } from "../api/client";
import type { AiProvider, AiProviderInput } from "../domain/types";

class AiProvidersStore {
  providers = $state<AiProvider[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  /** True bila minimal satu provider punya kunci di keychain → terjemahan siap. */
  get ready(): boolean {
    return this.providers.some((p) => p.has_key);
  }

  /** Jumlah provider dengan kunci tersimpan. */
  get readyCount(): number {
    return this.providers.filter((p) => p.has_key).length;
  }

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.providers = await api.aiProvidersList();
    } catch (e) {
      if (e instanceof Error && e.message === WEB_OFFLINE_MSG) {
        this.providers = [];
        return;
      }
      this.error = `gagal muat provider AI: ${e}`;
    } finally {
      this.loading = false;
    }
  }

  /** Simpan/upsert provider + kunci; melempar error agar form bisa menampilkan. */
  async save(provider: AiProviderInput, apiKey: string) {
    this.error = null;
    try {
      const saved = await api.aiProviderSave(provider, apiKey);
      this.providers = [
        ...this.providers.filter((p) => p.id !== saved.id),
        saved,
      ].sort((a, b) => a.name.localeCompare(b.name));
    } catch (e) {
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal simpan provider: ${e}`;
      }
      throw e;
    }
  }

  /** Hapus provider (metadata + kunci di backend); optimistik + rollback. */
  async remove(id: string) {
    this.error = null;
    const before = this.providers;
    this.providers = before.filter((p) => p.id !== id);
    try {
      await api.aiProviderDelete(id);
    } catch (e) {
      this.providers = before;
      if (!(e instanceof Error && e.message === WEB_OFFLINE_MSG)) {
        this.error = `gagal hapus provider: ${e}`;
      }
      throw e;
    }
  }
}

export const aiProvidersStore = new AiProvidersStore();
