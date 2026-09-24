import type { ReaderMode } from "../stores/settingsPersist";

export function paginate(
  current: number,
  total: number,
  delta: 1 | -1,
  rtl: boolean = false,
): number {
  if (total <= 0) return 1;
  const step = rtl ? -delta : delta;
  return Math.min(total, Math.max(1, current + step));
}

/** Nilai lama dipetakan ke 2 mode: semua scroll → vertical. */
export function defaultMode(raw: unknown): ReaderMode {
  if (raw === "paginated" || raw === "vertical") return raw;
  return "vertical";
}
