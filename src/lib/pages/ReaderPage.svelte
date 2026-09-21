<script lang="ts">
  // ReaderPage v0 — scroll vertikal + posisi tercatat ke riwayat (8.3).
  // Canvas 3-mode + virtual scroller + overlay tetap Fase 5 (7.x).
  // Perekaman ala mobile: buka = halaman 1, pindah halaman throttle 2 dtk.
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api } from "../api/client";
  import type { Chapter, Content, PageImageResult } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";

  let {
    content,
    chapter,
    source,
    onback,
  }: {
    content: Content;
    chapter: Chapter;
    source: string;
    onback: () => void;
  } = $props();

  let pages = $state<string[]>([]);
  let current = $state(1);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let lastRecord = 0;
  let listEl: HTMLElement | null = $state(null);

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

  $effect(() => {
    const ch = chapter;
    const src = source;
    loading = true;
    error = null;
    pages = [];
    current = 1;
    lastRecord = 0;
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
      { root: null, rootMargin: "-45% 0px -45% 0px", threshold: 0 }
    );
    imgs.forEach((img) => spy.observe(img));
    return () => spy.disconnect();
  });
</script>

<section>
  <header class="reader-head">
    <button class="ghost" onclick={goBack}>← Kembali</button>
    <div class="titles">
      <strong>{content.title}</strong>
      <span class="muted">{chapter.title || `Bab ${chapter.order}`}</span>
    </div>
    {#if pages.length > 0}
      <span class="badge">{current} / {pages.length}</span>
    {/if}
  </header>

  {#if loading}
    <p class="muted">Memuat halaman…</p>
  {/if}
  {#if error}
    <p class="err">{error}</p>
  {/if}
  {#if !loading && !error && pages.length === 0}
    <p class="muted">Tidak ada halaman untuk bab ini.</p>
  {/if}

  <div class="pages" bind:this={listEl}>
    {#each pages as src, i (i)}
      <img
        src={src}
        data-page={i + 1}
        alt={`Halaman ${i + 1}`}
        loading="lazy"
        draggable="false"
        referrerpolicy="no-referrer"
      />
    {/each}
  </div>
</section>

<style>
  .muted {
    color: var(--muted-foreground);
  }
  .err {
    color: var(--destructive);
  }
  .reader-head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    position: sticky;
    top: 0;
    z-index: 5;
    background: var(--background);
    padding: 8px 0;
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
  .ghost {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
    color: var(--foreground);
    cursor: pointer;
    flex-shrink: 0;
  }
  .badge {
    font-size: 12px;
    font-weight: 700;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--muted);
    color: var(--muted-foreground);
    flex-shrink: 0;
  }
  .pages {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .pages img {
    max-width: 800px;
    width: 100%;
    height: auto;
    border-radius: 8px;
    background: var(--muted);
  }
</style>
