// Test port 1:1 aturan lane bahasa mobile.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  buildChapterLanes,
  normalizeLaneKey,
  selectedLaneChapters,
  UNKNOWN_LANE,
} from "./chapterLanes.ts";
import type { Chapter } from "../domain/types.ts";

function ch(id: string, language?: string | null): Chapter {
  return {
    id,
    content_id: "m1",
    title: id,
    order: 1,
    is_external: false,
    external_url: null,
    language: language ?? null,
  };
}

describe("normalizeLaneKey", () => {
  it("peta nama panjang + varian ke kode", () => {
    assert.equal(normalizeLaneKey("English"), "en");
    assert.equal(normalizeLaneKey("eng"), "en");
    assert.equal(normalizeLaneKey("INDONESIAN"), "id");
    assert.equal(normalizeLaneKey("indo"), "id");
    assert.equal(normalizeLaneKey("Japanese"), "ja");
    assert.equal(normalizeLaneKey("zh-HK"), "zh");
    assert.equal(normalizeLaneKey("pt-BR"), "pt-br");
    assert.equal(normalizeLaneKey("es_LA"), "es-la");
  });

  it("kosong/null/unknown -> unknown", () => {
    assert.equal(normalizeLaneKey(null), UNKNOWN_LANE);
    assert.equal(normalizeLaneKey("  "), UNKNOWN_LANE);
    assert.equal(normalizeLaneKey("unknown"), UNKNOWN_LANE);
  });
});

describe("buildChapterLanes", () => {
  it("kelompok alfabet, unknown terakhir, default lane pertama", () => {
    const p = buildChapterLanes([
      ch("a", "id"),
      ch("b", null),
      ch("c", "en"),
      ch("d", "en"),
    ]);
    assert.deepEqual(
      p.lanes.map((l) => l.key),
      ["en", "id", "unknown"],
    );
    assert.equal(p.selectedKey, "en");
    assert.deepEqual(
      selectedLaneChapters(p).map((c) => c.id),
      ["c", "d"],
    );
  });

  it("pilihan user dipertahankan bila lane ada", () => {
    const p = buildChapterLanes([ch("a", "id"), ch("b", "en")], "id");
    assert.equal(p.selectedKey, "id");
    assert.deepEqual(
      selectedLaneChapters(p).map((c) => c.id),
      ["a"],
    );
  });

  it("pilihan tak dikenal -> fallback lane pertama; kosong -> null", () => {
    const p = buildChapterLanes([ch("a", "id")], "xx");
    assert.equal(p.selectedKey, "id");
    const empty = buildChapterLanes([]);
    assert.equal(empty.lanes.length, 0);
    assert.equal(empty.selectedKey, null);
    assert.deepEqual(selectedLaneChapters(empty), []);
  });

  it("satu lane unknown saja (scraper) -> tanpa pilihan ganda", () => {
    const p = buildChapterLanes([ch("a"), ch("b")]);
    assert.equal(p.lanes.length, 1);
    assert.equal(p.lanes[0].key, UNKNOWN_LANE);
    assert.equal(selectedLaneChapters(p).length, 2);
  });
});
