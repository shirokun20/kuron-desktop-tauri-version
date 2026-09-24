// Kontrak persist pengaturan (modul murni, storage mock).
// Jalankan: `pnpm test` (node:test bawaan, tanpa framework).
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_SETTINGS,
  SETTINGS_KEY,
  loadSettings,
  normalizeSettings,
  saveSettings,
} from "./settingsPersist.ts";

type MiniStorage = Pick<Storage, "getItem" | "setItem">;

function memStorage(seed?: Record<string, string>): MiniStorage {
  const m = new Map<string, string>(Object.entries(seed ?? {}));
  return {
    getItem: (k: string) => (m.has(k) ? m.get(k)! : null),
    setItem: (k: string, v: string) => {
      m.set(k, v);
    },
  } as MiniStorage;
}

describe("settingsPersist", () => {
  it("default: blur thumbnail ON, auto-load ON, reader vertical", () => {
    const s = memStorage();
    const d = loadSettings(s);
    assert.equal(d.blurThumbnail, true);
    assert.equal(d.autoLoadFeed, true);
    assert.equal(d.readerMode, "vertical");
    assert.equal(d.readerRightToLeft, false);
    assert.deepEqual(d, DEFAULT_SETTINGS);
  });

  it("simpan patch lalu muat ulang utuh (spec: persist setelah restart)", () => {
    const s = memStorage();
    saveSettings({ blurThumbnail: false, readerRightToLeft: true }, s);
    const d = loadSettings(s);
    assert.equal(d.blurThumbnail, false);
    assert.equal(d.readerRightToLeft, true);
    // Field tak disentuh tetap default.
    assert.equal(d.autoLoadFeed, true);
    assert.equal(d.readerMode, "vertical");
    // Kunci persis spec settings-ai.
    assert.ok(s.getItem(SETTINGS_KEY));
  });

  it("nilai rusak di storage jatuh ke default per-field", () => {
    const s = memStorage({
      [SETTINGS_KEY]: JSON.stringify({
        blurThumbnail: "yes",
        autoLoadFeed: false,
        readerMode: "bogus",
        extra: 1,
      }),
    });
    const d = loadSettings(s);
    assert.equal(d.blurThumbnail, true); // tipe salah → default ON
    assert.equal(d.autoLoadFeed, false); // boolean valid dipertahankan
    assert.equal(d.readerMode, "vertical"); // enum asing → default
    assert.equal("extra" in d, false); // key asing tak tembus
  });

  it("JSON rusak / array / kosong → default, tanpa throw", () => {
    assert.deepEqual(
      loadSettings(memStorage({ [SETTINGS_KEY]: "{oops" })),
      DEFAULT_SETTINGS,
    );
    assert.deepEqual(
      loadSettings(memStorage({ [SETTINGS_KEY]: "[1,2]" })),
      DEFAULT_SETTINGS,
    );
    assert.deepEqual(loadSettings(memStorage()), DEFAULT_SETTINGS);
    assert.deepEqual(normalizeSettings(null), DEFAULT_SETTINGS);
    assert.deepEqual(normalizeSettings([]), DEFAULT_SETTINGS);
  });

  it("save menormalkan patch yang salah tipe (pertahanan lapis-2)", () => {
    const s = memStorage();
    const bad = { readerMode: "x" } as unknown as Parameters<
      typeof saveSettings
    >[0];
    const d = saveSettings(bad, s);
    assert.equal(d.readerMode, "vertical");
    assert.equal(loadSettings(s).readerMode, "vertical");
  });

  it("nilai lama continuous/webtoon/spread/horizontal migrasi ke vertical", () => {
    for (const old of ["continuous", "webtoon", "spread", "horizontal"]) {
      const d = loadSettings(
        memStorage({ [SETTINGS_KEY]: JSON.stringify({ readerMode: old }) }),
      );
      assert.equal(d.readerMode, "vertical");
    }
  });

  it("tanpa storage = default + save tetap balik nilai utuh", () => {
    assert.deepEqual(loadSettings(undefined), DEFAULT_SETTINGS);
    const d = saveSettings({ blurThumbnail: false }, undefined);
    assert.equal(d.blurThumbnail, false);
  });
});
