<script lang="ts">
  // LibraryPage — Favorit & Riwayat dari SQLite backend (8.3).
  // Dibuka dari sidebar EXPLORE; data dari libraryStore (live singleton).
  import MainGridCard from "../components/MainGridCard.svelte";
  import { libraryStore } from "../stores/library.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import type { Content, HistoryItem } from "../domain/types";

  let {
    tab,
    onselect,
  }: { tab: string; onselect: (c: Content) => void } = $props();
  let isFav = $derived(tab === "favorites");

  /** Baris riwayat → Content sintetis agar bisa dibuka lagi di detail. */
  function historyContent(h: HistoryItem): Content {
    return {
      id: h.content_id,
      title: h.title,
      cover_url: h.cover_url,
      source_id: h.source_id,
      upload_date: null,
      is_favorite: false,
      page_count: null,
      language: null,
      tags: [],
      available_languages: [],
      description: null,
      rating: null,
      favorites: null,
    };
  }

  function fmtDate(epochSecs: bigint | number): string {
    const d = new Date(Number(epochSecs) * 1000);
    if (Number.isNaN(d.getTime())) return "";
    return d.toLocaleString("id-ID", {
      day: "numeric",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  async function resetLibrary() {
    if (!confirm("Reset library? Riwayat + favorit + snapshot dihapus.")) return;
    await libraryStore.clearLibrary();
  }
</script>

<section class="library">
  {#if libraryStore.loading && libraryStore.favorites.length === 0 && libraryStore.history.length === 0}
    <p class="muted">Memuat library…</p>
  {/if}
  {#if libraryStore.error}
    <p class="err">{libraryStore.error}</p>
  {/if}

  {#if isFav}
    <header class="lib-head halftone">
      <div>
        <p class="kicker">Koleksi</p>
        <h2>Galeri Favorit</h2>
      </div>
      <span class="count">{libraryStore.favorites.length}</span>
    </header>
    {#if libraryStore.favorites.length === 0 && !libraryStore.loading}
      <div class="empty">
        <p class="empty-title">Belum ada favorit</p>
        <p class="muted">Ketuk ikon hati di kartu untuk menyimpan ke sini.</p>
      </div>
    {:else}
      <div class="grid">
        {#each libraryStore.favorites as item (item.id)}
          <MainGridCard content={item} {onselect} />
        {/each}
      </div>
    {/if}
  {:else}
    <header class="lib-head halftone">
      <div>
        <p class="kicker">Jejak baca</p>
        <h2>Riwayat</h2>
      </div>
      <div class="head-actions">
        <span class="count">{libraryStore.history.length}</span>
        {#if libraryStore.history.length > 0}
          <button class="ghost danger" onclick={() => libraryStore.clearHistory()}>
            Hapus riwayat
          </button>
        {/if}
      </div>
    </header>
    {#if libraryStore.history.length === 0 && !libraryStore.loading}
      <div class="empty">
        <p class="empty-title">Riwayat kosong</p>
        <p class="muted">Tercatat otomatis saat membaca.</p>
      </div>
    {:else}
      <ul class="rows">
        {#each libraryStore.history as h (h.content_id)}
          <li class="row">
            <button
              class="open"
              onclick={() => onselect(historyContent(h))}
              aria-label={`Buka ${h.title}`}
            >
              {#if h.cover_url}
                <img
                  class:privacy={settingsStore.blurThumbnail}
                  src={h.cover_url}
                  alt=""
                  loading="lazy"
                  draggable="false"
                  referrerpolicy="no-referrer"
                />
              {:else}
                <span class="thumb-fallback">{h.title.slice(0, 2).toUpperCase()}</span>
              {/if}
              <span class="meta">
                <strong>{h.title}</strong>
                <span class="sub">
                  <span class="src">{h.source_id}</span>
                  <span class="muted">{fmtDate(h.updated_at)}</span>
                </span>
              </span>
              <span class="pos" title={`Terakhir di halaman ${h.position}`}>
                <span class="pos-num">{h.position}</span>
                <span class="pos-label">HAL</span>
              </span>
            </button>
            <button
              class="ghost kill"
              aria-label="Hapus dari riwayat"
              onclick={() => libraryStore.removeHistory(h.content_id)}
            >
              ✕
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  <div class="foot">
    <button class="ghost danger" onclick={resetLibrary}>
      Reset library
    </button>
  </div>
</section>

<style>
  .library {
    --ink-shadow: 4px 4px 0 rgb(0 0 0 / 0.35);
  }
  .muted {
    color: var(--muted-foreground);
  }
  .err {
    color: var(--destructive);
  }
  .halftone {
    background-image: radial-gradient(
      color-mix(in srgb, var(--foreground) 13%, transparent) 1.1px,
      transparent 1.3px
    );
    background-size: 12px 12px;
  }
  .lib-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    background-color: var(--card);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 18px 22px;
    margin-bottom: 20px;
  }
  .kicker {
    margin: 0 0 2px;
    font-family: "Komika", system-ui, sans-serif;
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .lib-head h2 {
    margin: 0;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 34px;
    font-weight: 400;
    letter-spacing: 0.03em;
    line-height: 1;
  }
  .count {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 34px;
    line-height: 1;
    color: var(--primary);
    background: var(--background);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 6px 16px;
    box-shadow: var(--ink-shadow);
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .empty {
    border: 1px dashed var(--border);
    border-radius: 14px;
    padding: 40px 24px;
    text-align: center;
  }
  .empty-title {
    margin: 0 0 6px;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 30px;
    letter-spacing: 0.03em;
  }
  .empty .muted {
    margin: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 16px;
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
  .ghost.danger {
    color: var(--destructive);
    border-color: color-mix(in srgb, var(--destructive) 45%, transparent);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    display: flex;
    align-items: stretch;
    gap: 8px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 8px;
  }
  .row:hover {
    border-color: var(--primary);
  }
  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--foreground);
    cursor: pointer;
    text-align: left;
  }
  .open img,
  .thumb-fallback {
    width: 56px;
    height: 74px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--muted);
    border: 1px solid var(--border);
    box-shadow: 3px 3px 0 rgb(0 0 0 / 0.3);
  }
  /* Privasi (9.1): thumb riwayat ikut blur thumbnail default-on. */
  .open img.privacy {
    filter: blur(8px);
  }
  .thumb-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 800;
    color: var(--muted-foreground);
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .meta strong {
    font-size: 15px;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
  }
  .src {
    font-family: "Komika", system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .pos {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-width: 64px;
    padding: 6px 10px;
    border-radius: 8px;
    background: var(--background);
    border: 1px solid var(--border);
  }
  .pos-num {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 28px;
    line-height: 1;
    color: var(--primary);
  }
  .pos-label {
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.2em;
    color: var(--muted-foreground);
  }
  .kill {
    align-self: center;
    flex-shrink: 0;
  }
  .foot {
    margin-top: 20px;
    display: flex;
    justify-content: flex-end;
  }
</style>
