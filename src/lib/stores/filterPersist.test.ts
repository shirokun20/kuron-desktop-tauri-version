// Kontrak simpanan filter per-sumber (modul murni, storage mock).
// Jalankan: `pnpm test` (node:test bawaan, tanpa framework).
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  clearSavedFilter,
  clearSavedQuery,
  loadSavedFilter,
  loadSavedQuery,
  saveFilter,
} from "./filterPersist.ts";

type MiniStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

function memStorage(): MiniStorage {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => (m.has(k) ? m.get(k)! : null),
    setItem: (k: string, v: string) => {
      m.set(k, v);
    },
    removeItem: (k: string) => {
      m.delete(k);
    },
  };
}

const MD = { title: {}, status: {}, includedTag: {} };
const NH = { query: {}, sort: {} };

describe("filterPersist", () => {
  it("simpan lalu muat kembali utuh", () => {
    const s = memStorage();
    saveFilter(
      "mangadex",
      { values: { title: "solo", status: "ongoing", includedTag: ["a", "b"] } },
      MD,
      s,
    );
    assert.deepEqual(loadSavedFilter("mangadex", MD, s)?.values, {
      title: "solo",
      status: "ongoing",
      includedTag: ["a", "b"],
    });
  });

  it("buang key asing, string kosong, dan tipe salah", () => {
    const s = memStorage();
    saveFilter(
      "mangadex",
      { values: { title: "  ", status: 42, includedTag: ["a", 7], asing: "x" } },
      MD,
      s,
    );
    assert.equal(loadSavedFilter("mangadex", MD, s), null);
  });

  it("query+values bergabung; ganti source saling lepas", () => {
    const s = memStorage();
    saveFilter("mangadex", { values: { title: "solo" } }, MD, s);
    saveFilter("mangadex", { query: "raw:title=solo", label: "solo" }, MD, s);
    saveFilter("nhentai", { values: { query: "desk" } }, NH, s);
    // Balik ke mangadex: values + query utuh.
    assert.deepEqual(loadSavedFilter("mangadex", MD, s), {
      values: { title: "solo" },
      query: "raw:title=solo",
      label: "solo",
    });
    // Nhentai: values ada, query belum ada.
    assert.deepEqual(loadSavedFilter("nhentai", NH, s)?.values, { query: "desk" });
    assert.equal(loadSavedQuery("nhentai", s), null);
    assert.deepEqual(loadSavedQuery("mangadex", s), {
      query: "raw:title=solo",
      label: "solo",
    });
  });

  it("autosave values tak hapus query; clear hapus semua", () => {
    const s = memStorage();
    saveFilter("mangadex", { query: "raw:title=solo", label: "solo" }, MD, s);
    saveFilter("mangadex", { values: { title: "solo leveling" } }, MD, s);
    assert.equal(loadSavedQuery("mangadex", s)?.query, "raw:title=solo");
    clearSavedFilter("mangadex", s);
    assert.equal(loadSavedFilter("mangadex", MD, s), null);
  });

  it("clearSavedQuery buang query saja, values tetap", () => {
    const s = memStorage();
    saveFilter("mangadex", { values: { title: "solo" } }, MD, s);
    saveFilter("mangadex", { query: "raw:title=solo", label: "solo" }, MD, s);
    clearSavedQuery("mangadex", s);
    assert.equal(loadSavedQuery("mangadex", s), null);
    assert.deepEqual(loadSavedFilter("mangadex", MD, s)?.values, { title: "solo" });
  });

  it("clearSavedQuery tanpa values = entri dihapus; tanpa storage no-op", () => {
    const s = memStorage();
    saveFilter("mangadex", { query: "raw:title=solo", label: "solo" }, MD, s);
    clearSavedQuery("mangadex", s);
    assert.equal(loadSavedFilter("mangadex", MD, s), null);
    clearSavedQuery("mangadex", undefined);
  });

  it("tanpa storage = no-op tanpa throw", () => {
    saveFilter("mangadex", { values: { title: "x" } }, MD, undefined);
    assert.equal(loadSavedFilter("mangadex", MD, undefined), null);
    assert.equal(loadSavedQuery("mangadex", undefined), null);
    clearSavedFilter("mangadex", undefined);
  });
});
