// Lane bahasa chapter — port 1:1 `chapter_language_presenter.dart` mobile.
// Aturan: normalisasi kode → kelompok per `chapter.language` → urut alfabet,
// "unknown" SELALU terakhir → pilihan default = lane pertama.
import type { Chapter } from "../domain/types";

export const UNKNOWN_LANE = "unknown";

export interface ChapterLane {
  key: string;
  chapters: Chapter[];
}

export interface ChapterLanes {
  lanes: ChapterLane[];
  selectedKey: string | null;
}

/** Normalisasi kode bahasa ala `ChapterLanguagePresenter.normalize`. */
export function normalizeLaneKey(value?: string | null): string {
  const raw = (value ?? "").trim().toLowerCase().replace(/_/g, "-");
  if (!raw) return UNKNOWN_LANE;
  if (raw === "pt-br" || raw === "es-la") return raw;
  const base = raw.split("-")[0];
  switch (base) {
    case "english":
    case "eng":
      return "en";
    case "indonesian":
    case "indo":
      return "id";
    case "japanese":
    case "jpn":
      return "ja";
    case "korean":
    case "kor":
      return "ko";
    case "chinese":
      return "zh";
    case "unknown":
      return UNKNOWN_LANE;
    default:
      return base;
  }
}

/** Kelompokkan chapter ke lane bahasa (urutan stabil per lane). */
export function buildChapterLanes(
  chapters: Chapter[],
  selectedKey?: string | null,
): ChapterLanes {
  const grouped = new Map<string, Chapter[]>();
  for (const ch of chapters) {
    const key = normalizeLaneKey(ch.language);
    const lane = grouped.get(key);
    if (lane) lane.push(ch);
    else grouped.set(key, [ch]);
  }
  const keys = [...grouped.keys()].sort((a, b) => {
    if (a === UNKNOWN_LANE) return 1;
    if (b === UNKNOWN_LANE) return -1;
    return a < b ? -1 : a > b ? 1 : 0;
  });
  const lanes: ChapterLane[] = keys.map((key) => ({
    key,
    chapters: grouped.get(key) ?? [],
  }));
  const normalized =
    selectedKey == null ? null : normalizeLaneKey(selectedKey);
  const effective =
    normalized != null && lanes.some((l) => l.key === normalized)
      ? normalized
      : (lanes.length > 0 ? lanes[0].key : null);
  return { lanes, selectedKey: effective };
}

/** Chapter pada lane terpilih (kosong bila tak ada lane). */
export function selectedLaneChapters(presentation: ChapterLanes): Chapter[] {
  if (presentation.selectedKey == null) {
    return presentation.lanes.flatMap((l) => l.chapters);
  }
  return (
    presentation.lanes.find((l) => l.key === presentation.selectedKey)
      ?.chapters ?? presentation.lanes[0]?.chapters ?? []
  );
}
