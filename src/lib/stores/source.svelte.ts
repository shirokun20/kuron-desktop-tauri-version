// Source store — sumber aktif, persist localStorage (dibaca kedua window).
export interface SourceMeta {
  id: string;
  label: string;
  version: string;
}

export const SOURCES = ["Semua", "NHentai", "Hitomi", "E-Hentai"];

export const SOURCE_META: SourceMeta[] = [
  { id: "nhentai", label: "NHentai", version: "v1.0.10" },
  { id: "hitomi", label: "Hitomi", version: "v1.2.0" },
  { id: "e-hentai", label: "E-Hentai", version: "v1.0.6" },
  { id: "mangadex", label: "MangaDex", version: "v1.1.10" },
];

const KEY = "kuron-source";

class SourceStore {
  current = $state<string>(localStorage.getItem(KEY) ?? "Semua");

  select(label: string) {
    this.current = label;
    localStorage.setItem(KEY, label);
  }
}

export const sourceStore = new SourceStore();
