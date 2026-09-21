<script lang="ts">
  // ReaderPage v0 — ruang baca tinta: progres + bab sebelum/berikutnya.
  // Canvas 3-mode + virtual scroller + overlay tetap Fase 5 (7.x).
  // Perekaman ala mobile: buka = halaman 1, pindah halaman throttle 2 dtk.
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api } from "../api/client";
  import type { Chapter, Content, PageImageResult } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";

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
  let lastRecord = 0;
  let listEl: HTMLElement | null = $state(null);
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

  /** Scroller milik MainPage (`.main-col`): gulir ke atas tiap ganti bab. */
  function scrollTop() {
    document.querySelector(".main-col")?.scrollTo({ top: 0 });
  }

  $effect(() => {
    const ch = chapter;
    const src = source;
    loading = true;
    error = null;
    pages = [];
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

  // Mata-mata halaman: gambar di tengah viewport = posisi baca.
  $effect(() => {
    if (!listEl || pages.length === 0) return;
    const imgs = listEl.querySelectorAll("img[data-page]");
    const spy = new IntersectionObserver(
      (entries) => {
        for (const en of entries) {
          if (!en.isIntersecting) continue;
          const page = Number((en.target as HTMLElement).dataset.page);
          if (Number.isInteger(page) && page !== current) {
            current = page;
            recordThrottled(page);
          }
        }
      },
      { root: null, rootMargin: "-45% 0px -45% 0px", threshold: 0 },
    );
    imgs.forEach((img) => spy.observe(img));
    return () => spy.disconnect();
  });
</script>

<section class="reader">
  <header class="reader-head">
    <button class="ghost" onclick={goBack}>←</button>
    <div class="titles">
      <strong>{content.title}</strong>
      <span class="muted">{chapter.title || `Bab ${chapter.order}`}</span>
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

  <div class="pages" bind:this={listEl}>
    {#each pages as src, i (i)}
      <figure>
        <img
          src={src}
          data-page={i + 1}
          alt={`Halaman ${i + 1}`}
          loading="lazy"
          draggable="false"
          referrerpolicy="no-referrer"
        />
        <figcaption>{i + 1}</figcaption>
      </figure>
    {/each}
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
  .reader {
    background: var(--kuron-reader-bg);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 0 20px 24px;
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
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 -20px 18px;
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
  .pages {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .pages figure {
    margin: 0;
    width: 100%;
    max-width: 800px;
  }
  .pages img {
    display: block;
    width: 100%;
    height: auto;
    border-radius: 4px;
    background: var(--muted);
    border: 1px solid var(--border);
  }
  .pages figcaption {
    margin-top: 4px;
    text-align: center;
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 14px;
    letter-spacing: 0.1em;
    color: var(--muted-foreground);
  }
  .reader-foot {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    max-width: 800px;
    margin: 20px auto 0;
  }
  .reader-foot .ghost {
    max-width: 48%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
