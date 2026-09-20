// Simpanan filter per-sumber — `localStorage` (`kuron.filter.<source>`).
// `values` = restore isi form; `query`+`label` = auto-apply saat ganti source.
export type FilterValues = Record<string, string | string[]>;

export interface SavedFilter {
  values: FilterValues;
  query?: string;
  label?: string;
}

type MiniStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

function defaultStorage(): MiniStorage | undefined {
  return typeof localStorage !== "undefined" ? localStorage : undefined;
}

export function filterKey(source: string): string {
  return `kuron.filter.${source}`;
}

/** Buang key asing, string kosong, dan array kosong/bukan-string. */
export function cleanValues(
  values: Record<string, unknown>,
  params: Record<string, unknown>,
): FilterValues {
  const out: FilterValues = {};
  for (const [k, v] of Object.entries(values)) {
    if (!(k in params)) continue;
    if (typeof v === "string") {
      if (v.trim()) out[k] = v;
    } else if (Array.isArray(v) && v.every((x) => typeof x === "string")) {
      const a = (v as string[]).filter((x) => x.trim() !== "");
      if (a.length) out[k] = a;
    }
  }
  return out;
}

function readRaw(source: string, storage?: MiniStorage): Record<string, unknown> | null {
  const s = storage ?? defaultStorage();
  if (!s) return null;
  try {
    const raw = s.getItem(filterKey(source));
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return null;
    return parsed as Record<string, unknown>;
  } catch {
    return null;
  }
}

/** Muat simpanan (values divalidasi vs params kini). */
export function loadSavedFilter(
  source: string,
  params: Record<string, unknown>,
  storage?: MiniStorage,
): SavedFilter | null {
  const raw = readRaw(source, storage);
  if (!raw) return null;
  const values = cleanValues(
    (raw.values as Record<string, unknown> | undefined) ?? {},
    params,
  );
  const query =
    typeof raw.query === "string" && raw.query.trim() ? raw.query : undefined;
  const label = typeof raw.label === "string" && raw.label ? raw.label : undefined;
  if (!query && Object.keys(values).length === 0) return null;
  return { values, query, label };
}

/** Query tersimpan saja (auto-apply ganti source, tanpa butuh params). */
export function loadSavedQuery(
  source: string,
  storage?: MiniStorage,
): { query: string; label: string } | null {
  const raw = readRaw(source, storage);
  if (!raw) return null;
  if (typeof raw.query !== "string" || !raw.query.trim()) return null;
  const label = typeof raw.label === "string" && raw.label ? raw.label : raw.query;
  return { query: raw.query, label };
}

/** Hapus kueri tersimpan (query+label), `values` form dibiarkan. */
export function clearSavedQuery(source: string, storage?: MiniStorage): void {
  const s = storage ?? defaultStorage();
  if (!s) return;
  const raw = readRaw(source, s);
  if (!raw) return;
  if (raw.query === undefined && raw.label === undefined) return;
  const values =
    raw.values && typeof raw.values === "object" && !Array.isArray(raw.values)
      ? (raw.values as FilterValues)
      : {};
  try {
    if (Object.keys(values).length === 0) {
      s.removeItem(filterKey(source));
    } else {
      s.setItem(filterKey(source), JSON.stringify({ values }));
    }
  } catch {
    // abaikan: quota / private mode
  }
}

/** Tulis gabung (patch menimpa per-field); kosong total = hapus key. */
export function saveFilter(
  source: string,
  patch: { values?: Record<string, unknown>; query?: string; label?: string },
  params: Record<string, unknown>,
  storage?: MiniStorage,
): void {
  const s = storage ?? defaultStorage();
  if (!s) return;
  const prev = loadSavedFilter(source, params, s) ?? { values: {} as FilterValues };
  const values =
    patch.values !== undefined ? cleanValues(patch.values, params) : prev.values;
  const query = patch.query !== undefined ? patch.query : prev.query;
  const label = patch.label !== undefined ? patch.label : prev.label;
  try {
    if (!query && Object.keys(values).length === 0) {
      s.removeItem(filterKey(source));
    } else {
      s.setItem(filterKey(source), JSON.stringify({ values, query, label }));
    }
  } catch {
    // abaikan: quota / private mode
  }
}

export function clearSavedFilter(source: string, storage?: MiniStorage): void {
  try {
    (storage ?? defaultStorage())?.removeItem(filterKey(source));
  } catch {
    // abaikan
  }
}
