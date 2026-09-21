<script lang="ts">
  // MainGridCard — ala kartu mobile: cover full-bleed, scrim bawah
  // (keterbacaan teks), judul 2 baris + badge halaman & bahasa.
  import type { Content } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";
  import { langFlag, langLabel } from "../utils/lang";

  let { content }: { content: Content } = $props();
  let imgFailed = $state(false);
  let fav = $derived(libraryStore.isFav(content.id));

  const BOOK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>';
  const HEART = '<svg viewBox="0 0 24 24" fill="FILL" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/></svg>';
</script>

<article class="grid-card">
  <div class="cover">
    <button
      class="fav"
      class:on={fav}
      aria-label={fav ? "Hapus favorit" : "Tandai favorit"}
      aria-pressed={fav}
      onclick={(e) => {
        e.stopPropagation();
        libraryStore.toggleFav(content);
      }}
    >
      {@html HEART.replace("FILL", fav ? "currentColor" : "none")}
    </button>
    {#if content.cover_url && !imgFailed}
      <img
        src={content.cover_url}
        alt={content.title}
        loading="lazy"
        draggable="false"
        referrerpolicy="no-referrer"
        onerror={() => (imgFailed = true)}
      />
    {:else}
      <span class="fallback">{content.title.slice(0, 2).toUpperCase()}</span>
    {/if}
    <div class="scrim">
      <h3>{content.title}</h3>
      <div class="badges">
        {#if content.page_count}
          <span class="badge pages"><span class="ic">{@html BOOK}</span>{content.page_count}</span>
        {/if}
        {#if content.language}
          <span class="badge lang">{langFlag(content.language)} {langLabel(content.language)}</span>
        {/if}
      </div>
    </div>
  </div>
</article>

<style>
  .grid-card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 16px;
    overflow: hidden;
    transition: border-color 150ms ease;
  }
  .grid-card:hover {
    border-color: var(--primary);
  }
  .cover {
    position: relative;
    aspect-ratio: 3 / 4;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--muted);
  }
  .cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .fallback {
    font-size: 32px;
    font-weight: 800;
    color: var(--muted-foreground);
    opacity: 0.5;
  }
  .fav {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
    cursor: pointer;
    opacity: 0;
    transition: opacity 150ms ease, color 150ms ease, border-color 150ms ease;
  }
  .grid-card:hover .fav,
  .fav.on {
    opacity: 1;
  }
  .fav.on {
    color: var(--primary);
    border-color: color-mix(in srgb, var(--primary) 60%, transparent);
  }
  .fav :global(svg) {
    width: 15px;
    height: 15px;
  }
  .scrim {
    position: absolute;
    inset: auto 0 0 0;
    padding: 28px 12px 10px;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.85) 30%, transparent);
  }
  .scrim h3 {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
    color: #fff;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .badges {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 999px;
  }
  .badge.pages {
    background: color-mix(in srgb, var(--primary) 85%, transparent);
    color: #fff;
  }
  .badge.lang {
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
  }
  .ic {
    display: inline-flex;
  }
  .ic :global(svg) {
    width: 12px;
    height: 12px;
  }
</style>
