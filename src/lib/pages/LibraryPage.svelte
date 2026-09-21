<script lang="ts">
  // LibraryPage — daftar Favorit & Riwayat dari SQLite backend (8.3).
  // Dibuka dari sidebar EXPLORE; data dari libraryStore (live singleton).
  import MainGridCard from "../components/MainGridCard.svelte";
  import { libraryStore } from "../stores/library.svelte";
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

<section>
  {#if libraryStore.loading && libraryStore.favorites.length === 0 && libraryStore.history.length === 0}
    <p class="muted">Memuat library…</p>
  {/if}
  {#if libraryStore.error}
    <p class="err">{libraryStore.error}</p>
  {/if}

  {#if isFav}
    <h2>Galeri favorit ({libraryStore.favorites.length})</h2>
    {#if libraryStore.favorites.length === 0 && !libraryStore.loading}
      <p class="muted">Belum ada favorit — ketuk ikon hati di kartu.</p>
    {:else}
      <div class="grid">
        {#each libraryStore.favorites as item (item.id)}
          <MainGridCard content={item} {onselect} />
        {/each}
      </div>
    {/if}
  {:else}
    <div class="rowhead">
      <h2>Riwayat ({libraryStore.history.length})</h2>
      {#if libraryStore.history.length > 0}
        <button class="ghost danger" onclick={() => libraryStore.clearHistory()}>
          Hapus riwayat
        </button>
      {/if}
    </div>
    {#if libraryStore.history.length === 0 && !libraryStore.loading}
      <p class="muted">Riwayat kosong — tercatat saat membaca.</p>
    {:else}
      <ul class="rows">
        {#each libraryStore.history as h (h.content_id)}
          <li class="row">
            {#if h.cover_url}
              <img
                src={h.cover_url}
                alt=""
                loading="lazy"
                draggable="false"
                referrerpolicy="no-referrer"
              />
            {:else}
              <span class="thumb-fallback">{h.title.slice(0, 2).toUpperCase()}</span>
            {/if}
            <div
              class="meta clickable"
              role="button"
              tabindex="0"
              onclick={() => onselect(historyContent(h))}
              onkeydown={(e) => {
                if (e.key === "Enter") onselect(historyContent(h));
              }}
            >
              <strong>{h.title}</strong>
              <span class="muted">
                {h.source_id} · halaman {h.position} · {fmtDate(h.updated_at)}
              </span>
            </div>
            <button
              class="ghost"
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
  h2 {
    margin: 4px 0 16px;
    font-size: 18px;
    letter-spacing: -0.02em;
  }
  .muted {
    color: var(--muted-foreground);
  }
  .err {
    color: var(--destructive);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 16px;
  }
  .rowhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .rowhead h2 {
    margin: 4px 0 16px;
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
    align-items: center;
    gap: 12px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 8px 12px 8px 8px;
  }
  .row img,
  .thumb-fallback {
    width: 44px;
    height: 58px;
    border-radius: 8px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--muted);
  }
  .thumb-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 800;
    color: var(--muted-foreground);
  }
  .row .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row .meta.clickable {
    cursor: pointer;
  }
  .row .meta strong {
    font-size: 14px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .row .meta .muted {
    font-size: 12px;
  }
  .foot {
    margin-top: 20px;
    display: flex;
    justify-content: flex-end;
  }
</style>
