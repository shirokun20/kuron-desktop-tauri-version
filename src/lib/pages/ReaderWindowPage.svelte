<script lang="ts">
  // ReaderWindowPage — isi popup window native `#reader` (ala #about).
  // Window baru = store kosong, jadi parameter (source/content/chapter/lang)
  // dibaca dari query hash lalu detail + daftar bab diambil sendiri dari
  // backend. Tutup = tutup window; pindah bab = state lokal popup.
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type { Chapter, Content } from "../domain/types";
  import ReaderPage from "./ReaderPage.svelte";

  let content = $state<Content | null>(null);
  let chapter = $state<Chapter | null>(null);
  let siblings = $state<Chapter[]>([]);
  let source = $state("");
  let error = $state<string | null>(null);

  function readParams(): {
    source: string;
    contentId: string;
    chapterId: string;
    lang: string | null;
  } | null {
    const h = window.location.hash;
    if (!h.startsWith("#reader")) return null;
    const q = h.slice("#reader".length);
    const p = new URLSearchParams(q.startsWith("?") ? q.slice(1) : q);
    const s = p.get("source");
    const c = p.get("content");
    const ch = p.get("chapter");
    if (!s || !c || !ch) return null;
    return { source: s, contentId: c, chapterId: ch, lang: p.get("lang") };
  }

  onMount(() => {
    const params = readParams();
    if (!params) {
      error = "parameter baca tak lengkap — buka bab dari halaman detail.";
      return;
    }
    source = params.source;
    let cancelled = false;
    (async () => {
      try {
        const d = await api.detail(params.contentId, params.source);
        if (cancelled) return;
        content = d;
        // Bahasa awal 1:1 DetailPage: `en` bila ada, else pertama.
        const langs = d.available_languages ?? [];
        const lang =
          params.lang ??
          (langs.length === 0
            ? null
            : langs.includes("en")
              ? "en"
              : langs[0]);
        const list = await api.chapters(
          params.contentId,
          params.source,
          lang,
        );
        if (cancelled) return;
        siblings = [...list].sort((a, b) => a.order - b.order);
        const found = siblings.find((c) => c.id === params.chapterId) ?? null;
        if (!found) {
          error = "bab tak ketemu di daftar sumber — buka ulang dari detail.";
          return;
        }
        chapter = found;
      } catch (e) {
        if (!cancelled) error = `gagal muat bacaan: ${e}`;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  function close() {
    try {
      void getCurrentWindow().close().catch(() => window.close());
    } catch {
      window.close();
    }
  }
</script>

{#if error}
  <main class="win-err">
    <h1>Gagal buka bacaan</h1>
    <p>{error}</p>
    <button type="button" onclick={close}>Tutup</button>
  </main>
{:else if content && chapter}
  <ReaderPage
    {content}
    {chapter}
    {siblings}
    {source}
    onback={close}
    onchapter={(ch) => {
      chapter = ch;
    }}
  />
{:else}
  <main class="win-load">
    <p>Memuat bacaan…</p>
  </main>
{/if}

<style>
  .win-load,
  .win-err {
    min-height: 100dvh;
    display: grid;
    place-content: center;
    gap: 12px;
    text-align: center;
    padding: 24px;
  }
</style>
