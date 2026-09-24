<script lang="ts">
  // ReaderPage — ruang baca tinta: progres + bab sebelum/berikutnya.
  // Render 3-mode via ReaderCanvas (7.1); overlay translate + draw tetap 7.2/7.3.
  // Perekaman ala mobile: buka = halaman 1, pindah halaman throttle 2 dtk.
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api } from "../api/client";
  import type { Chapter, Content, PageImageResult } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import ReaderCanvas from "../components/ReaderCanvas.svelte";
  import { defaultMode } from "../utils/readerLayout";
  import type { ReaderMode } from "../stores/settingsPersist";

  let {
    content,
    chapter,
    siblings,
    source,
    onback,
    onchapter,
  }: {
    content: Content;
    chapter: Chapter;
    siblings: Chapter[];
    source: string;
    onback: () => void;
    onchapter: (ch: Chapter) => void;
  } = $props();

  let pages = $state<string[]>([]);
  let current = $state(1);
  let loading = $state(true);
  let error = $state<string | null>(null);
  /** Halaman yang gagal dimuat (1-based) — tiap tile bawa tombol reload. */
  let failed = $state(new Set<number>());
  let lastRecord = 0;
  let rootEl: HTMLElement | null = $state(null);
  /** Pilihan user terakhir — langsung persist ke Pengaturan (storage). */
  let mode: ReaderMode = $derived(
    defaultMode(settingsStore.s.readerMode),
  );

  function pickMode(m: ReaderMode) {
    settingsStore.patch({ readerMode: m });
  }
  let rtl = $derived(settingsStore.s.readerRightToLeft);
  // Bab internal berurutan (eksternal dibuka di browser, bukan di sini).
  let readable = $derived(siblings.filter((c) => !c.is_external));
  let atIndex = $derived(readable.findIndex((c) => c.id === chapter.id));
  let prev = $derived(atIndex > 0 ? readable[atIndex - 1] : null);
  let next = $derived(
    atIndex >= 0 && atIndex < readable.length - 1
      ? readable[atIndex + 1]
      : null,
  );
  let progress = $derived(pages.length > 0 ? current / pages.length : 0);

  function srcOf(p: PageImageResult): string {
    return p.kind === "Cached" ? convertFileSrc(p.value) : p.value;
  }

  /** Rekam max 1× per 2 dtk (throttle ala mobile D.3). */
  function recordThrottled(page: number) {
    const now = Date.now();
    if (now - lastRecord < 2000) return;
    lastRecord = now;
    libraryStore.recordHistory(content, page);
  }

  function goBack() {
    // Flush posisi terakhir sebelum keluar (fire-and-forget).
    libraryStore.recordHistory(content, current);
    onback();
  }

  function goChapter(ch: Chapter) {
    libraryStore.recordHistory(content, current);
    onchapter(ch);
  }

  /** Route khusus = scroller milik sendiri: atas tiap ganti bab.
      Vertical: scroll milik `.canvas-wrap`; paged: tak perlu scroll. */
  function scrollTop() {
    rootEl?.querySelector(".canvas-wrap")?.scrollTo({ top: 0 });
  }

  function retryFailed() {
    failed = new Set();
  }

  function retryPage(p: number) {
    failed.delete(p);
    failed = new Set(failed);
  }

  function markFailed(p: number) {
    failed.add(p);
    failed = new Set(failed);
  }

  /** Gambar sukses tampil → bersihkan flag gagal sesaat (pulih otomatis). */
  function markOk(p: number) {
    if (failed.has(p)) {
      failed.delete(p);
      failed = new Set(failed);
    }
  }

  function trackPage(p: number) {
    if (p === current) return;
    current = p;
    recordThrottled(p);
  }

  $effect(() => {
    const ch = chapter;
    const src = source;
    loading = true;
    error = null;
    pages = [];
    failed = new Set();
    current = 1;
    lastRecord = 0;
    scrollTop();
    let cancelled = false;
    // Backend: kirim `external_url` penuh bila ada (komentar impl).
    api
      .pageImages(ch.external_url ?? ch.id, src)
      .then((res) => {
        if (cancelled) return;
        pages = res.map(srcOf);
        if (pages.length > 0) {
          lastRecord = Date.now();
          libraryStore.recordHistory(content, 1);
        }
      })
      .catch((e) => {
        if (!cancelled) error = `gagal muat halaman: ${e}`;
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  // Mata-mata halaman kini di dalam ReaderCanvas (lapor via onpage).
</script>

<section class="reader" bind:this={rootEl}>
  <header class="reader-head">
    <button class="ghost" onclick={goBack}>←</button>
    <div class="titles">
      <strong>{content.title}</strong>
      <span class="muted">{chapter.title || `Bab ${chapter.order}`}</span>
    </div>
    {#if failed.size > 0}
      <button class="ghost retry-all" onclick={retryFailed}>
        ⟳ {failed.size} gagal
      </button>
    {/if}
    <div class="modeseg" role="group" aria-label="Mode baca">
      <button
        type="button"
        class:on={mode === "paginated"}
        aria-pressed={mode === "paginated"}
        title="Per halaman (tombol/keyboard)"
        onclick={() => pickMode("paginated")}
      >
        ▦
      </button>
      <button
        type="button"
        class:on={mode === "vertical"}
        aria-pressed={mode === "vertical"}
        title="Gulir atas-bawah (gambar tinggi = scroll panjang alami)"
        onclick={() => pickMode("vertical")}
      >
        ≣
      </button>
    </div>
    <div class="chapnav">
      <button
        class="ghost nav"
        disabled={!prev}
        title={prev ? `Sebelumnya: ${prev.title}` : "Bab pertama"}
        onclick={() => prev && goChapter(prev)}
      >
        ‹
      </button>
      <button
        class="ghost nav"
        disabled={!next}
        title={next ? `Berikutnya: ${next.title}` : "Bab terakhir"}
        onclick={() => next && goChapter(next)}
      >
        ›
      </button>
    </div>
    {#if pages.length > 0}
      <span class="badge">{current} / {pages.length}</span>
    {/if}
    <div
      class="progress"
      style:width={`${Math.round(progress * 100)}%`}
      aria-hidden="true"
    ></div>
  </header>

  {#if loading}
    <p class="muted center">Memuat halaman…</p>
  {/if}
  {#if error}
    <p class="err">{error}</p>
  {/if}
  {#if !loading && !error && pages.length === 0}
    <p class="muted center">Tidak ada halaman untuk bab ini.</p>
  {/if}

  <div class="canvas-wrap" class:scroll={mode === "vertical"}>
    <ReaderCanvas
      {pages}
      {mode}
      {rtl}
      {current}
      {failed}
      onpage={trackPage}
      onfail={markFailed}
      onretry={retryPage}
      onok={markOk}
    />
  </div>

  {#if !loading && pages.length > 0}
    <footer class="reader-foot">
      <button class="ghost" disabled={!prev} onclick={() => prev && goChapter(prev)}>
        ← {prev ? prev.title : "Awal"}
      </button>
      <button class="ghost" disabled={!next} onclick={() => next && goChapter(next)}>
        {next ? next.title : "Akhir"} →
      </button>
    </footer>
  {/if}
</section>

<style>
  /* Shell flex penuh: header + kanvas + footer menempel bingkai bawah.
     AKAR "space antara tombol prev/next dengan frame": footer mengalir
     setelah konten (bukan sticky) + kanvas menebak tinggi viewport
     (`100dvh−300px`), sehingga ada ruang kosong di bawah gambar. */
  .reader {
    background: var(--kuron-reader-bg);
    height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 0 20px 10px;
  }
  .canvas-wrap {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  /* Mode vertical: kanvas ikut scroll shell (satu scroll saja, mulus). */
  .canvas-wrap.scroll {
    overflow-y: auto;
    display: block;
    scroll-behavior: smooth;
  }
  .muted {
    color: var(--muted-foreground);
  }
  .center {
    text-align: center;
  }
  .err {
    color: var(--destructive);
  }
  .reader-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 -20px 10px;
    padding: 10px 20px;
    position: sticky;
    top: 0;
    z-index: 5;
    background: var(--kuron-reader-bg);
    border-bottom: 1px solid var(--border);
    border-radius: 14px 14px 0 0;
  }
  .titles {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .titles strong {
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .titles .muted {
    font-size: 12px;
  }
  .chapnav {
    display: flex;
    gap: 6px;
  }
  .ghost {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 13px;
    font-size: 14px;
    font-weight: 700;
    color: var(--foreground);
    cursor: pointer;
    flex-shrink: 0;
  }
  .ghost.nav {
    font-size: 18px;
    line-height: 1;
    padding: 5px 12px 8px;
  }
  .ghost:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .retry-all {
    border-color: var(--destructive);
    color: var(--destructive);
    font-size: 12px;
    padding: 6px 12px;
  }
  .modeseg {
    flex-shrink: 0;
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--card);
  }
  .modeseg button {
    border: 0;
    background: transparent;
    color: var(--muted-foreground);
    font-size: 15px;
    padding: 6px 11px;
    cursor: pointer;
  }
  .modeseg button + button {
    border-left: 1px solid var(--border);
  }
  .modeseg button.on {
    background: var(--primary);
    color: var(--primary-foreground);
  }
  .badge {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 17px;
    letter-spacing: 0.08em;
    padding: 4px 14px;
    border-radius: 6px;
    background: var(--primary);
    color: var(--primary-foreground);
    flex-shrink: 0;
  }
  .progress {
    position: absolute;
    left: 0;
    bottom: -1px;
    height: 2px;
    background: var(--primary);
    transition: width 200ms ease;
  }
  /* Footer menempel bingkai bawah app (bukan mengalir setelah konten)
     + merge dengan tombol Awal/Akhir via space-between penuh. */
  .reader-foot {
    flex-shrink: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 8px 0 2px;
    background: var(--kuron-reader-bg);
    border-top: 1px solid var(--border);
    margin-top: 8px;
  }
  .reader-foot .ghost {
    max-width: 48%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
