import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  isRectLikePolygon,
  inscribedBox,
  fitFontSize,
  bubbleToScreen,
} from "./translateOverlay.ts";

describe("isRectLikePolygon", () => {
  it("persegi terdeteksi rect-like", () => {
    assert.equal(
      isRectLikePolygon([
        [0, 0],
        [100, 0],
        [100, 50],
        [0, 50],
      ]),
      true,
    );
  });
  it("oval tidak rect-like", () => {
    const pts: [number, number][] = [];
    for (let i = 0; i < 16; i++) {
      const a = (i / 16) * Math.PI * 2;
      pts.push([50 + 50 * Math.cos(a), 25 + 25 * Math.sin(a)]);
    }
    assert.equal(isRectLikePolygon(pts), false);
  });
  it("degenerate (<3 titik) false", () => {
    assert.equal(isRectLikePolygon([[0, 0]]), false);
  });
});

describe("inscribedBox", () => {
  it("susut 10% per sisi", () => {
    const b = inscribedBox([
      [0, 0],
      [100, 0],
      [100, 50],
      [0, 50],
    ]);
    assert(Math.abs(b.left - 10) < 1e-6);
    assert(Math.abs(b.top - 5) < 1e-6);
    assert(Math.abs(b.width - 80) < 1e-6);
    assert(Math.abs(b.height - 40) < 1e-6);
  });
});

describe("fitFontSize", () => {
  it("bubble besar = font besar", () => {
    assert(fitFontSize("Halo dunia", 300, 150, false) > 12);
  });
  it("teks panjang di kotak kecil = font kecil", () => {
    const s = fitFontSize(
      "Kalimat yang sangat panjang sekali dan terus berlanjut tanpa henti",
      60,
      40,
      false,
    );
    assert(s <= 10);
  });
  it("box nol = minimal", () => {
    assert.equal(fitFontSize("x", 0, 0, false), 6);
  });
});

describe("bubbleToScreen", () => {
  it("fitWidth skala benar", () => {
    const b = bubbleToScreen(
      { x: 100, y: 200, w: 300, h: 150 },
      1000,
      2000,
      500,
      1000,
      0,
    );
    assert(Math.abs(b.left - 50) < 1e-6);
    assert(Math.abs(b.top - 100) < 1e-6);
    assert(Math.abs(b.width - 150) < 1e-6);
    assert(Math.abs(b.height - 75) < 1e-6);
  });
});
