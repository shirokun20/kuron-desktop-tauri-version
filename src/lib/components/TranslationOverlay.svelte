<script lang="ts">
  // TranslationOverlay — lapisan bubble terjemahan di atas gambar reader (7.2).
  // Port `ReaderTranslationOverlay` + `_BubbleShapePainter` + `_fitText`:
  // poligon halus (bezier midpoint; rect-like = tepi lurus), halo putih
  // feather + inti pekat, teks hitam outline putih fit-inscribed.
  // Self-measuring: absolute inset-0 dalam figure relative; koordinat bubble
  // (px asli) dipetakan ke ukuran render sendiri via ResizeObserver.
  // Pointer-events none — baca tetap milik kanvas di bawahnya.
  import { onMount } from "svelte";
  import type { TranslatedBubble } from "../domain/types";
  import {
    fitFontSize,
    inscribedBox,
    isRectLikePolygon,
    type Pt,
  } from "../utils/translateOverlay";

  let {
    bubbles,
    imgW,
    imgH,
  }: {
    bubbles: TranslatedBubble[];
    /** Dimensi gambar asli (px) — ruang koordinat bubble. */
    imgW: number;
    /** Dimensi gambar asli (px). */
    imgH: number;
  } = $props();

  let el: HTMLDivElement | null = $state(null);
  let cw = $state(0);
  let ch = $state(0);

  onMount(() => {
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect;
      if (r) {
        cw = r.width;
        ch = r.height;
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  });

  /** Poligon absolut → path SVG halus (bezier midpoint ala mobile). */
  function smoothPath(pts: Pt[]): string {
    const n = pts.length;
    const mid = (a: Pt, b: Pt): Pt => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    const start = mid(pts[n - 1], pts[0]);
    let d = `M ${start[0].toFixed(1)} ${start[1].toFixed(1)}`;
    for (let i = 0; i < n; i++) {
      const ctrl = pts[i];
      const m = mid(ctrl, pts[(i + 1) % n]);
      d += ` Q ${ctrl[0].toFixed(1)} ${ctrl[1].toFixed(1)} ${m[0].toFixed(1)} ${m[1].toFixed(1)}`;
    }
    return d + " Z";
  }

  function straightPath(pts: Pt[]): string {
    return "M " + pts.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join(" L ") + " Z";
  }

  /** Susutkan poligon ke centroid 0.8× (inti pekat ala `_deflate`). */
  function deflate(pts: Pt[]): Pt[] {
    const n = pts.length;
    const cx = pts.reduce((s, p) => s + p[0], 0) / n;
    const cy = pts.reduce((s, p) => s + p[1], 0) / n;
    return pts.map(([x, y]) => [cx + (x - cx) * 0.8, cy + (y - cy) * 0.8] as Pt);
  }

  interface Placed {
    left: number;
    top: number;
    width: number;
    height: number;
    text: string;
    font: number;
    whitePatch: boolean;
    halo: string | null;
    core: string | null;
    tx: number;
    ty: number;
    tw: number;
    th: number;
  }

  let placed = $derived.by((): Placed[] => {
    if (imgW <= 0 || imgH <= 0 || cw <= 0 || ch <= 0) return [];
    // Kontainer kini pas gambar (rasio template) — skala langsung.
    const sx = cw / imgW;
    const sy = ch / imgH;
    const out: Placed[] = [];
    for (const b of bubbles) {
      const left = b.x * sx;
      const top = b.y * sy;
      const width = Math.max(1, b.w * sx);
      const height = Math.max(1, b.h * sy);
      const shape = b.shape && b.shape.length >= 3 ? (b.shape as Pt[]) : null;
      let halo: string | null = null;
      let core: string | null = null;
      let tw = width;
      let th = height;
      let tx = 0;
      let ty = 0;
      if (shape) {
        // Absolut dalam koordinat overlay (relatif rect bubble).
        const abs = shape.map(([x, y]) => [x * sx, y * sy] as Pt);
        const local = abs.map(([x, y]) => [x - left, y - top] as Pt);
        const path = (pts: Pt[]) =>
          isRectLikePolygon(
            pts.map(([x, y]) => [x - left, y - top] as Pt),
          )
            ? straightPath(pts)
            : smoothPath(pts);
        halo = path(abs);
        core = path(deflate(abs));
        const ib = inscribedBox(local);
        tw = Math.max(8, ib.width);
        th = Math.max(8, ib.height);
        tx = ib.left;
        ty = ib.top;
      }
      out.push({
        left,
        top,
        width,
        height,
        text: b.translated,
        font: fitFontSize(b.translated, tw, th, shape !== null),
        whitePatch: !shape && b.needs_white_patch,
        halo,
        core,
        tx,
        ty,
        tw,
        th,
      });
    }
    return out;
  });
</script>

<div class="tl-overlay" bind:this={el} aria-hidden="true">
  {#each placed as pl, i (i)}
    <div
      class="tl-bubble"
      style:left={`${pl.left.toFixed(1)}px`}
      style:top={`${pl.top.toFixed(1)}px`}
      style:width={`${pl.width.toFixed(1)}px`}
      style:height={`${pl.height.toFixed(1)}px`}
    >
      {#if pl.halo && pl.core}
        <svg
          class="tl-svg"
          viewBox={`0 0 ${pl.width.toFixed(1)} ${pl.height.toFixed(1)}`}
        >
          <g transform={`translate(${-pl.left.toFixed(1)},${-pl.top.toFixed(1)})`}>
            <path
              d={pl.halo}
              fill="rgba(255,255,255,0.55)"
              stroke="none"
              style="filter: blur(4px)"
            />
            <path d={pl.core} fill="rgba(255,255,255,0.92)" stroke="none" />
          </g>
        </svg>
      {:else if pl.whitePatch}
        <div class="tl-patch"></div>
      {/if}
      <div
        class="tl-text"
        style:left={`${pl.tx.toFixed(1)}px`}
        style:top={`${pl.ty.toFixed(1)}px`}
        style:width={`${Math.max(1, pl.tw).toFixed(1)}px`}
        style:height={`${Math.max(1, pl.th).toFixed(1)}px`}
        style:font-size={`${pl.font}px`}
      >
        <span>{pl.text}</span>
      </div>
    </div>
  {/each}
</div>

<style>
  .tl-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .tl-bubble {
    position: absolute;
  }
  .tl-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .tl-patch {
    position: absolute;
    inset: 0;
    background: rgba(255, 255, 255, 0.9);
    border-radius: 4px;
  }
  .tl-text {
    position: absolute;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 1px;
  }
  /* Teks manga: hitam + outline putih 8-arah (stroke), bukan halo blur. */
  .tl-text span {
    color: #111;
    font-weight: 600;
    line-height: 1.25;
    font-family: "Komika", "KosugiMaru", system-ui, sans-serif;
    text-shadow:
      -1px -1px 0 #fff, 0 -1px 0 #fff, 1px -1px 0 #fff,
      -1px 0 0 #fff, 1px 0 0 #fff,
      -1px 1px 0 #fff, 0 1px 0 #fff, 1px 1px 0 #fff;
  }
</style>
