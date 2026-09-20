// Source store — daftar DINAMIS dari backend (`cmd_sources_list`, 4.7).
// `current` = id sumber ("semua" = mock agregat). Label untuk display.
import { api } from "../api/client";

export interface SourceMeta {
  id: string;
  label: string;
  version: string;
  iconUrl?: string | null;
}

const KEY = "kuron-source";

function prettyLabel(id: string): string {
  if (id === "semua") return "Semua";
  if (id === "nhentai") return "NHentai";
  if (id === "e-hentai") return "E-Hentai";
  return id.charAt(0).toUpperCase() + id.slice(1);
}

/** Samakan id lama (label) ke id baru. */
function migrate(raw: string | null): string {
  if (!raw) return "semua";
  const lower = raw.toLowerCase();
  if (lower === "semua") return "semua";
  if (lower === "nhentai") return "nhentai";
  if (lower === "hitomi") return "hitomi";
  if (lower === "e-hentai") return "e-hentai";
  if (lower === "mangadex") return "mangadex";
  return lower;
}

class SourceStore {
  available = $state<SourceMeta[]>([
    { id: "semua", label: "Semua", version: "mock" },
  ]);
  current = $state<string>(migrate(localStorage.getItem(KEY)));
  loading = $state(false);
  error = $state<string | null>(null);

  get currentLabel(): string {
    return (
      this.available.find((s) => s.id === this.current)?.label ??
      prettyLabel(this.current)
    );
  }

  /** Muat daftar dari backend; fallback daftar lokal bila gagal. */
  async load() {
    this.loading = true;
    this.error = null;
    try {
      const list = await api.sourcesList();
      const metas: SourceMeta[] = [
        { id: "semua", label: "Semua", version: "mock" },
        ...list.map((s) => ({
          id: s.id,
          label: prettyLabel(s.id),
          version: s.version ? `v${s.version}` : "",
          iconUrl: s.icon_url,
        })),
      ];
      this.available = metas;
      const known = metas.some((s) => s.id === this.current);
      // Backend menjawab daftar (non-kosong) tapi id tersimpan tak ada di
      // sana → fallback "semua". Daftar kosong (mode web tanpa backend)
      // JANGAN mereset pilihan user.
      if (!known && list.length > 0) {
        this.select("semua");
      }
    } catch (e) {
      this.error = `gagal muat sumber: ${e}`;
    } finally {
      this.loading = false;
    }
  }

  select(id: string) {
    this.current = id;
    localStorage.setItem(KEY, id);
  }
}

export const sourceStore = new SourceStore();
