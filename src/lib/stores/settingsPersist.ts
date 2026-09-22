// Preferensi pengaturan aplikasi — `localStorage` (`kuron.settings`),
// pola `filterPersist` (modul murni, testable, storage disuntik).
// Spec: `settings-ai` — seksi tampilan/pembaca/jaringan/data; blur default-on.
export type ReaderMode = "paginated" | "continuous" | "webtoon";

export interface AppSettings {
  /** Kaburkan cover demi privasi — default ON (ala mobile). */
  blurThumbnail: boolean;
  /** Muat feed otomatis saat app dibuka / sumber diganti. */
  autoLoadFeed: boolean;
  /** Mode baca default — dikonsumsi ReaderCanvas (tugas 7.1). */
  readerMode: ReaderMode;
  /** Arah baca kanan-ke-kiri — dikonsumsi ReaderCanvas (tugas 7.1). */
  readerRightToLeft: boolean;
}

export const SETTINGS_KEY = "kuron.settings";

export const DEFAULT_SETTINGS: AppSettings = {
  blurThumbnail: true,
  autoLoadFeed: true,
  readerMode: "continuous",
  readerRightToLeft: false,
};

const READER_MODES: readonly ReaderMode[] = [
  "paginated",
  "continuous",
  "webtoon",
];

type MiniStorage = Pick<Storage, "getItem" | "setItem">;

function defaultStorage(): MiniStorage | undefined {
  return typeof localStorage !== "undefined" ? localStorage : undefined;
}

/** Objek sembarang → `AppSettings` utuh; nilai salah tipe/asing → default. */
export function normalizeSettings(raw: unknown): AppSettings {
  const out: AppSettings = { ...DEFAULT_SETTINGS };
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return out;
  const o = raw as Record<string, unknown>;
  if (typeof o.blurThumbnail === "boolean") out.blurThumbnail = o.blurThumbnail;
  if (typeof o.autoLoadFeed === "boolean") out.autoLoadFeed = o.autoLoadFeed;
  if (typeof o.readerRightToLeft === "boolean") {
    out.readerRightToLeft = o.readerRightToLeft;
  }
  if (
    typeof o.readerMode === "string" &&
    (READER_MODES as readonly string[]).includes(o.readerMode)
  ) {
    out.readerMode = o.readerMode as ReaderMode;
  }
  return out;
}

export function loadSettings(storage?: MiniStorage): AppSettings {
  const s = storage ?? defaultStorage();
  if (!s) return { ...DEFAULT_SETTINGS };
  try {
    const raw = s.getItem(SETTINGS_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return normalizeSettings(JSON.parse(raw));
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

/** Patch di atas simpanan kini; hasil utuh dipersist + dikembalikan. */
export function saveSettings(
  patch: Partial<AppSettings>,
  storage?: MiniStorage,
): AppSettings {
  const next = normalizeSettings({ ...loadSettings(storage), ...patch });
  const s = storage ?? defaultStorage();
  if (s) {
    try {
      s.setItem(SETTINGS_KEY, JSON.stringify(next));
    } catch {
      // quota / private mode — nilai hasil tetap berlaku sesi ini
    }
  }
  return next;
}
