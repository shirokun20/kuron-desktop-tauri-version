// Geometri overlay terjemahan — port `polygon_geometry.dart` +
// `_inscribedBox` + `_fitText` dari `reader_translation_widgets.dart` (7.2).
// Semua murni (testable): poligon layar → inscribed box → ukuran font pas.

export type Pt = [number, number];

/** True bila poligon nyaris persegi sejajar sumbu (shoelace ≥0.95 bbox).
 *  Rect-like = tepi lurus; else = smoothing bezier oval. */
export function isRectLikePolygon(points: Pt[]): boolean {
  if (points.length < 3) return false;
  let left = points[0][0];
  let right = points[0][0];
  let top = points[0][1];
  let bottom = points[0][1];
  let doubleArea = 0;
  for (let i = 0; i < points.length; i++) {
    const p = points[i];
    if (p[0] < left) left = p[0];
    if (p[0] > right) right = p[0];
    if (p[1] < top) top = p[1];
    if (p[1] > bottom) bottom = p[1];
    const q = points[(i + 1) % points.length];
    doubleArea += p[0] * q[1] - q[0] * p[1];
  }
  const boxW = right - left;
  const boxH = bottom - top;
  if (boxW < 4 || boxH < 4) return false;
  const area = Math.abs(doubleArea) / 2;
  if (area <= 0) return false;
  return area / (boxW * boxH) >= 0.95;
}

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Bbox poligon disusut ~0.8× (±10% per sisi) — teks bungkus tetap di
 *  dalam lengkung oval, bukan meluber ke sudut (port `_inscribedBox`). */
export function inscribedBox(points: Pt[]): Box {
  let left = points[0][0];
  let right = points[0][0];
  let top = points[0][1];
  let bottom = points[0][1];
  for (const p of points.slice(1)) {
    if (p[0] < left) left = p[0];
    if (p[0] > right) right = p[0];
    if (p[1] < top) top = p[1];
    if (p[1] > bottom) bottom = p[1];
  }
  const shrinkW = (right - left) * 0.1;
  const shrinkH = (bottom - top) * 0.1;
  return {
    left: left + shrinkW,
    top: top + shrinkH,
    width: right - left - shrinkW * 2,
    height: bottom - top - shrinkH * 2,
  };
}

/** Perkiraan kasar lebar teks: ~0.55em per glyph (CJK ~1em). */
function estimateWidth(text: string, size: number): number {
  let w = 0;
  for (const ch of text) {
    w += ch.codePointAt(0)! > 0x2e7f ? size : size * 0.55;
  }
  return w;
}

/** Bungkus kata sederhana pada maxWidth; kembalikan jumlah baris + max baris. */
function wrapLines(text: string, size: number, maxW: number): { lines: number; widest: number } {
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length === 0) return { lines: 1, widest: 0 };
  let lines = 1;
  let cur = 0;
  let widest = 0;
  for (const w of words) {
    const ww = estimateWidth(w, size);
    const gap = cur > 0 ? size * 0.3 : 0;
    if (cur + gap + ww <= maxW || cur === 0) {
      cur += gap + ww;
    } else {
      widest = Math.max(widest, cur);
      lines++;
      cur = ww;
    }
  }
  return { lines, widest: Math.max(widest, cur) };
}

/** Font-fit: ukuran terbesar (turun) yang muat dalam box ala cypy/mobile.
 *  [6..32] skala bubble kecil; tinggi baris 1.25. */
export function fitFontSize(
  text: string,
  boxW: number,
  boxH: number,
  hasShape: boolean,
): number {
  if (boxW <= 0 || boxH <= 0) return 6;
  const shortSide = Math.min(boxW, boxH);
  const scale = shortSide < 40 ? shortSide / 40 : 1;
  const maxSize = Math.min(32, Math.max(6, 32 * scale));
  const minSize = Math.min(7, Math.max(4, 7 * scale));
  const maxW = boxW * (hasShape ? 0.98 : 0.8);
  const maxH = boxH * (hasShape ? 0.98 : 0.9);
  for (let size = Math.floor(maxSize); size >= Math.ceil(minSize); size--) {
    const { lines, widest } = wrapLines(text, size, maxW);
    if (widest <= maxW && lines * size * 1.25 <= maxH) return size;
  }
  return Math.max(4, Math.ceil(minSize));
}

/** Petakan bubble piksel-asli → rect layar (fitWidth + offset vertikal). */
export function bubbleToScreen(
  bubble: { x: number; y: number; w: number; h: number },
  imgW: number,
  imgH: number,
  screenW: number,
  renderedH: number,
  topOffset: number,
): Box {
  const scaleX = screenW / imgW;
  const scaleY = renderedH / imgH;
  return {
    left: bubble.x * scaleX,
    top: bubble.y * scaleY + topOffset,
    width: bubble.w * scaleX,
    height: bubble.h * scaleY,
  };
}
