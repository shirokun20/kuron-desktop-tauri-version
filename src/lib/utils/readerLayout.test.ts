import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { defaultMode, paginate } from "./readerLayout.ts";

describe("paginate", () => {
  it("maju/mundur dijepit", () => {
    assert.equal(paginate(1, 5, 1), 2);
    assert.equal(paginate(5, 5, 1), 5);
    assert.equal(paginate(1, 5, -1), 1);
  });
  it("rtl membalik arah", () => {
    assert.equal(paginate(2, 5, 1, true), 1);
    assert.equal(paginate(2, 5, -1, true), 3);
  });
});

describe("defaultMode", () => {
  it("2 mode valid + semua nilai lama ke vertical", () => {
    assert.equal(defaultMode("paginated"), "paginated");
    assert.equal(defaultMode("vertical"), "vertical");
    for (const old of ["continuous", "webtoon", "spread", "horizontal", "x"]) {
      assert.equal(defaultMode(old), "vertical");
    }
  });
});
