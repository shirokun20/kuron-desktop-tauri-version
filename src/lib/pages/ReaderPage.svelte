<script lang="ts">
  // ReaderPage — ruang baca tinta: progres + bab sebelum/berikutnya.
  // Render 3-mode via ReaderCanvas (7.1); overlay translate + draw tetap 7.2/7.3.
  // Perekaman ala mobile: buka = halaman 1, pindah halaman throttle 2 dtk.
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { api } from "../api/client";
  import { isTauriRuntime } from "../api/platform";
  import type { Chapter, Content, PageImageResult } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { translateStore } from "../stores/translate.svelte";
  import { aiProvidersStore } from "../stores/aiProviders.svelte";
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
  /** Nilai mentah backend per halaman (URL remote / path lokal) — untuk translate. */
  let rawPages = $state<string[]>([]);
  let current = $state(1);
  let loading = $state(true);
  let error = $state<string | null>(null);
  /** Halaman yang gagal dimuat (1-based) — tiap tile bawa tombol reload. */
  let failed = $state(new Set<number>());
  let lastRecord = 0;
  let rootEl: HTMLElement | null = $state(null);

  onMount(() => {
    translateStore.startListening();
    if (isTauriRuntime()) void aiProvidersStore.load();
    return () => translateStore.stopListening();
  });

  /** Deteksi bubble halaman kini (tombol 🛰, manual ala draw-mode).
   *  Sekali per halaman; gagal = banner error, baca tetap normal. */
  function detectCurrent() {
    const p = current;
    const url = rawPages[p - 1];
    if (!url || translateStore.detecting.has(p)) return;
    void translateStore.detectPage(url, source, p);
  }

  /** Terjemahkan halaman kini (toolbar ✨). Backend ambil bytes sendiri. */
  async function translateCurrent() {
    const p = current;
    const url = rawPages[p - 1];
    if (!url || translateStore.busy) return;
    try {
      await translateStore.translatePage({
        pageUrl: url,
        sourceId: source,
        contentId: content.id,
        pageIndex: p - 1,
        rtl,
        page: p,
      });
    } catch {
      // stage + error sudah di store; banner di bawah header menampilkannya
    }
  }

  function tlBanner(): string | null {
    if (translateStore.stage === "no-provider")
      return "Belum ada provider AI — tambah kunci di Pengaturan → AI · Terjemahan.";
    if (translateStore.stage === "rate-limited")
      return "Provider rate-limited (429) — tunggu ±60 dtk atau ganti model.";
    if (translateStore.stage === "error" && translateStore.error)
      return `Terjemahan gagal: ${translateStore.error}`;
    // Error deteksi manual (bukan dari stage translate): tampilkan juga.
    if (
      !translateStore.busy &&
      !translateStore.resultFor(current) &&
      translateStore.error
    )
      return `Deteksi gagal: ${translateStore.error}`;
    return null;
  }
  /** Pilihan user terakhir — langsung persist ke Pengaturan (storage). */
  let mode: ReaderMode = $derived(
    defaultMode(settingsStore.s.readerMode),
  );

  function pickMode(m: ReaderMode) {
    settingsStore.patch({ readerMode: m });
  }
  let rtl = $derived(settingsStore.s.readerRightToLeft);
  /** Jumlah bubble terdeteksi halaman kini (badge tombol ✨). */
  let nBox = $derived(translateStore.boxesFor(current).length);
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
    rawPages = [];
    failed = new Set();
    current = 1;
    lastRecord = 0;
    translateStore.resetPage();
    scrollTop();
    let cancelled = false;
    // Backend: kirim `external_url` penuh bila ada (komentar impl).
    api
      .pageImages(ch.external_url ?? ch.id, src)
      .then((res) => {
        if (cancelled) return;
        // Mentah untuk translate (path lokal / URL); tampil via srcOf.
        rawPages = res.map((p) => p.value);
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
        title={prev ? `Bab sebelumnya: ${prev.title}` : "Bab pertama"}
        aria-label={prev ? `Bab sebelumnya: ${prev.title}` : "Bab pertama"}
        onclick={() => prev && goChapter(prev)}
      >
        «
      </button>
      <button
        class="ghost nav"
        disabled={!next}
        title={next ? `Bab berikutnya: ${next.title}` : "Bab terakhir"}
        aria-label={next ? `Bab berikutnya: ${next.title}` : "Bab terakhir"}
        onclick={() => next && goChapter(next)}
      >
        »
      </button>
    </div>
    <div class="aibar" role="group" aria-label="Terjemahan AI">
      <button
        type="button"
        class="ghost ai"
        class:on={nBox > 0}
        disabled={translateStore.detecting.has(current) || pages.length === 0}
        title={nBox > 0
          ? `${nBox} bubble terdeteksi — klik ✨ untuk terjemahkan`
          : "Deteksi bubble halaman ini"}
        onclick={detectCurrent}
      >
        {#if translateStore.detecting.has(current)}
          <span class="spin" aria-hidden="true"></span>
        {:else}
          🛰{nBox > 0 ? nBox : ""}
        {/if}
      </button>
      <button
        type="button"
        class="ghost ai"
        class:on={translateStore.overlayVisible &&
          !!translateStore.resultFor(current)}
        disabled={translateStore.busy || pages.length === 0}
        title={translateStore.resultFor(current)
          ? "Tampilkan/sembunyikan terjemahan"
          : nBox > 0
            ? `Terjemahkan ${nBox} bubble halaman ini`
            : "Terjemahkan halaman ini"}
        onclick={() => {
          if (translateStore.resultFor(current)) translateStore.toggleOverlay();
          else void translateCurrent();
        }}
      >
        {#if translateStore.busy}
          <span class="spin" aria-hidden="true"></span>
        {:else}
          ✨{nBox > 0 && !translateStore.resultFor(current) ? nBox : ""}
        {/if}
      </button>
      {#if translateStore.resultFor(current)}
        <button
          type="button"
          class="ghost ai clear"
          title="Hapus terjemahan halaman ini"
          onclick={() => translateStore.clearResult(current)}
        >
          🗑
        </button>
      {/if}
      {#if nBox > 0}
        <!-- Bersihkan outline biru halaman ini (deteksi ulang bila perlu). -->
        <button
          type="button"
          class="ghost ai clear"
          title={`Bersihkan ${nBox} bubble halaman ini`}
          aria-label={`Bersihkan ${nBox} bubble halaman ${current}`}
          onclick={() => translateStore.clearDetected(current)}
        >
          🧹
        </button>
      {/if}
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
  {#if tlBanner()}
    <p class="tl-banner" role="status">{tlBanner()}</p>
  {:else if translateStore.stage === "detecting"}
    <p class="tl-banner busy" role="status">Mendeteksi bubble…</p>
  {:else if translateStore.stage === "translating"}
    <p class="tl-banner busy" role="status">Menerjemahkan halaman {current}…</p>
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
      translations={translateStore.results}
      detected={translateStore.detected}
      showTl={translateStore.overlayVisible}
    />
  </div>

  {#if !loading && pages.length > 0}
    <!-- SATU baris sejajar: bab-kiri | halaman-tengah | bab-kanan.
         Nav halaman di dalam footer permanen (bukan ikut scroll gambar)
         agar tak hilang saat gambar besar. -->
    <footer class="reader-foot">
      <button
        class="ghost chap"
        disabled={!prev}
        title={prev ? `Bab sebelumnya: ${prev.title}` : "Bab pertama"}
        aria-label={prev ? `Bab sebelumnya: ${prev.title}` : "Bab pertama"}
        onclick={() => prev && goChapter(prev)}
      >
        « <span class="kick">Bab</span>
        <span class="blabel">{prev ? prev.title : "Awal"}</span>
      </button>
      {#if mode === "paginated"}
        <nav class="pagefoot" aria-label="Navigasi halaman">
          <button
            class="ghost"
            disabled={current <= 1}
            title={`Halaman sebelumnya (${Math.max(1, current - 1)} / ${pages.length})`}
            aria-label={`Halaman sebelumnya (${Math.max(1, current - 1)} / ${pages.length})`}
            onclick={() => trackPage(Math.max(1, current - 1))}
          >
            ‹ <span class="blabel">Sebelumnya</span>
          </button>
          <span class="pos">{current} / {pages.length}</span>
          <button
            class="ghost"
            disabled={current >= pages.length}
            title={`Halaman berikutnya (${Math.min(pages.length, current + 1)} / ${pages.length})`}
            aria-label={`Halaman berikutnya (${Math.min(pages.length, current + 1)} / ${pages.length})`}
            onclick={() => trackPage(Math.min(pages.length, current + 1))}
          >
            <span class="blabel">Berikutnya</span> ›
          </button>
        </nav>
      {/if}
      <button
        class="ghost chap"
        disabled={!next}
        title={next ? `Bab berikutnya: ${next.title}` : "Bab terakhir"}
        aria-label={next ? `Bab berikutnya: ${next.title}` : "Bab terakhir"}
        onclick={() => next && goChapter(next)}
      >
        <span class="kick">Bab</span>
        <span class="blabel">{next ? next.title : "Akhir"}</span> »
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
  /* Toolbar translate AI (7.2): tombol ✨ + hapus hasil halaman. */
  .aibar {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .ghost.ai {
    font-size: 16px;
    padding: 6px 11px 8px;
  }
  .ghost.ai.on {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
  }
  .ghost.ai.clear {
    border-color: var(--destructive);
    color: var(--destructive);
  }
  .spin {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid var(--muted-foreground);
    border-top-color: transparent;
    border-radius: 50%;
    animation: aispin 0.8s linear infinite;
    vertical-align: -2px;
  }
  @keyframes aispin {
    to {
      transform: rotate(360deg);
    }
  }
  .tl-banner {
    flex-shrink: 0;
    margin: 0 0 8px;
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 13px;
    background: color-mix(in srgb, var(--destructive) 10%, transparent);
    border: 1px solid var(--destructive);
    color: var(--foreground);
  }
  .tl-banner.busy {
    background: var(--card);
    border-color: var(--border);
    color: var(--muted-foreground);
  }
  .progress {
    position: absolute;
    left: 0;
    bottom: -1px;
    height: 2px;
    background: var(--primary);
    transition: width 200ms ease;
  }
  /* Footer SATU baris: Awal | nav-halaman tengah | Akhir.
     Menempel bingkai bawah (bukan ikut scroll gambar). */
  .reader-foot {
    flex-shrink: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    row-gap: 6px;
    flex-wrap: wrap;
    padding: 8px 0 2px;
    background: var(--kuron-reader-bg);
    border-top: 1px solid var(--border);
    margin-top: 8px;
  }
  .pagefoot {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 10px;
    flex: 1 1 auto;
    min-width: 0;
  }
  .pagefoot .pos {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 15px;
    letter-spacing: 0.08em;
    color: var(--muted-foreground);
    flex-shrink: 0;
  }
  .reader-foot .ghost {
    max-width: 48%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Kicker "Bab" di tombol bab — bedakan dari tombol halaman
     (Sebelumnya/Berikutnya tanpa kicker). */
  .ghost.chap .kick {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--primary);
    margin-right: 2px;
  }
  .ghost.chap:disabled .kick {
    color: var(--muted-foreground);
  }
  /* Sempit: nav halaman turun di bawah + label panjang disembunyikan
     (ikon panah + kicker Bab tetap sebagai penanda) — tak berdesakan. */
  @media (max-width: 640px) {
    .reader-foot {
      justify-content: center;
    }
    .pagefoot {
      order: 3;
      flex-basis: 100%;
    }
    .reader-foot .ghost .blabel {
      display: none;
    }
    .reader-foot .ghost {
      max-width: none;
    }
  }
</style>
