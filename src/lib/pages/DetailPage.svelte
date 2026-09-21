<script lang="ts">
  // DetailPage — info konten + daftar chapter (v0, prasyarat riwayat 8.3).
  // Chapter internal → reader in-app; eksternal → browser via opener.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../api/client";
  import type { Chapter, Content } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";
  import { langFlag, langLabel } from "../utils/lang";

  let {
    content,
    onback,
    onopenchapter,
  }: {
    content: Content;
    onback: () => void;
    onopenchapter: (c: Chapter) => void;
  } = $props();

  let fetched = $state<Content | null>(null);
  let chapters = $state<Chapter[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let fav = $derived(libraryStore.isFav(content.id));
  // Prop sebagai tampilan awal; hasil fetch menimpanya saat tiba.
  let detail = $derived(fetched ?? content);

  $effect(() => {
    const c = content;
    loading = true;
    error = null;
    fetched = null;
    chapters = [];
    let cancelled = false;
    (async () => {
      try {
        const [d, ch] = await Promise.all([
          api.detail(c.id, c.source_id),
          api.chapters(c.id, c.source_id),
        ]);
        if (cancelled) return;
        fetched = d;
        chapters = [...ch].sort((a, b) => a.order - b.order);
      } catch (e) {
        if (!cancelled) error = `gagal muat detail: ${e}`;
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  async function openChapter(ch: Chapter) {
    // Eksternal (baca di situs asal) → browser; sisanya reader in-app.
    if (ch.is_external) {
      if (ch.external_url) await openUrl(ch.external_url);
      return;
    }
    onopenchapter(ch);
  }
</script>

<section>
  <button class="ghost" onclick={onback}>← Kembali</button>

  {#if loading}
    <p class="muted">Memuat detail…</p>
  {/if}
  {#if error}
    <p class="err">{error}</p>
  {/if}

  <div class="hero">
    {#if detail.cover_url}
      <img
        src={detail.cover_url}
        alt=""
        draggable="false"
        referrerpolicy="no-referrer"
      />
    {:else}
      <span class="cover-fallback">{detail.title.slice(0, 2).toUpperCase()}</span>
    {/if}
    <div class="info">
      <h2>{detail.title}</h2>
      <p class="muted">{detail.source_id}</p>
      <div class="badges">
        {#if detail.page_count}
          <span class="badge">{detail.page_count} hal</span>
        {/if}
        {#if detail.language}
          <span class="badge">{langFlag(detail.language)} {langLabel(detail.language)}</span>
        {/if}
        {#if detail.upload_date}
          <span class="badge">{detail.upload_date}</span>
        {/if}
      </div>
      <div class="actions">
        <button
          class="primary"
          disabled={chapters.length === 0}
          onclick={() => chapters.length > 0 && openChapter(chapters[0])}
        >
          Mulai Baca
        </button>
        <button class="ghost" onclick={() => libraryStore.toggleFav(detail)}>
          {fav ? "♥ Favorit" : "♡ Favorit"}
        </button>
      </div>
    </div>
  </div>

  <h3>Bab ({chapters.length})</h3>
  {#if !loading && chapters.length === 0 && !error}
    <p class="muted">Belum ada bab untuk konten ini.</p>
  {:else}
    <ul class="rows">
      {#each chapters as ch (ch.id)}
        <li>
          <button class="row" onclick={() => openChapter(ch)}>
            <span class="order">{ch.order}</span>
            <span class="title">{ch.title || `Bab ${ch.order}`}</span>
            {#if ch.is_external}
              <span class="badge ext">eksternal ↗</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .muted {
    color: var(--muted-foreground);
  }
  .err {
    color: var(--destructive);
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
  }
  .primary {
    background: var(--primary);
    border: 1px solid var(--primary);
    border-radius: 10px;
    padding: 7px 16px;
    font-size: 13px;
    font-weight: 700;
    color: #fff;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .hero {
    display: flex;
    gap: 20px;
    margin: 16px 0 24px;
  }
  .hero img,
  .cover-fallback {
    width: 160px;
    height: 214px;
    border-radius: 12px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--muted);
  }
  .cover-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 36px;
    font-weight: 800;
    color: var(--muted-foreground);
  }
  .info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .info h2 {
    margin: 0;
    font-size: 20px;
    letter-spacing: -0.02em;
  }
  .info p {
    margin: 0;
    font-size: 13px;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .badge {
    font-size: 11px;
    font-weight: 700;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--muted);
    color: var(--muted-foreground);
  }
  .badge.ext {
    background: transparent;
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  h3 {
    font-size: 15px;
    margin: 0 0 12px;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 10px 14px;
    font-size: 14px;
    color: var(--foreground);
    cursor: pointer;
    text-align: left;
  }
  .row:hover {
    border-color: var(--primary);
  }
  .row .order {
    font-weight: 800;
    color: var(--primary);
    min-width: 28px;
  }
  .row .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
