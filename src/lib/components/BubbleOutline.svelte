<script lang="ts">
  // BubbleOutline — outline biru bubble terdeteksi SEBELUM diterjemahkan (7.2).
  // Port draw-mode Detect mobile: referensi biru hasil ONNX, tanpa teks.
  // Self-measuring seperti TranslationOverlay (koordinat px asli → render).
  // Pointer-events none — baca tetap milik kanvas di bawahnya.
  import { onMount } from "svelte";
  import type { BubbleBox } from "../domain/types";
  import { isRectLikePolygon, type Pt } from "../utils/translateOverlay";

  let {
    boxes,
    imgW,
    imgH,
  }: {
    boxes: BubbleBox[];
    /** Dimensi gambar asli (px) — ruang koordinat deteksi. */
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

  interface Outline {
    d: string;
    rect: { left: number; top: number; width: number; height: number } | null;
  }

  let outlines = $derived.by((): Outline[] => {
    if (imgW <= 0 || imgH <= 0 || cw <= 0 || ch <= 0) return [];
    // Kontainer kini pas gambar (rasio template) — skala langsung.
    const sx = cw / imgW;
    const sy = ch / imgH;
    return boxes.map((b) => {
      const left = b.x * sx;
      const top = b.y * sy;
      const width = Math.max(1, b.w * sx);
      const height = Math.max(1, b.h * sy);
      const shape = b.shape && b.shape.length >= 3 ? (b.shape as Pt[]) : null;
      if (shape) {
        const abs = shape.map(([x, y]) => [x * sx, y * sy] as Pt);
        const local = abs.map(([x, y]) => [x - left, y - top] as Pt);
        const d = isRectLikePolygon(local) ? straightPath(abs) : smoothPath(abs);
        return { d, rect: null };
      }
      return { d: "", rect: { left, top, width, height } };
    });
  });
</script>

<div class="bl-overlay" bind:this={el} aria-hidden="true">
  <svg class="bl-svg" viewBox={`0 0 ${cw} ${ch}`} preserveAspectRatio="none">
    {#each outlines as o, i (i)}
      {#if o.d}
        <path d={o.d} class="bl-path" />
      {:else if o.rect}
        <rect
          x={o.rect.left}
          y={o.rect.top}
          width={o.rect.width}
          height={o.rect.height}
          rx="6"
          class="bl-path"
        />
      {/if}
    {/each}
  </svg>
</div>

<style>
  .bl-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .bl-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  /* Outline biru referensi (draw-mode Detect mobile): stroke tegas +
     halo putih tipis agar terbaca di atas artwork apa pun. */
  .bl-path {
    fill: rgba(47, 123, 255, 0.07);
    stroke: #2f7bff;
    stroke-width: 2;
    stroke-linejoin: round;
    paint-order: stroke;
  }
</style>
