<script lang="ts">
  // SourceList — daftar sumber (tile, nama, id • versi, hint, check/chevron).
  // Dipakai popover sheet maupun popup window.
  import { tagColor } from "../theme/tokens";
  import { themeStore } from "../stores/theme.svelte";
  import type { SourceMeta } from "../stores/source.svelte";

  interface Props {
    sources: SourceMeta[];
    /** id sumber aktif. */
    active: string;
    onSelect: (id: string) => void;
  }

  let { sources, active, onSelect }: Props = $props();
  let query = $state("");
  let iconFailed = $state<Record<string, boolean>>({});
  let filtered = $derived(
    sources.filter((s) =>
      `${s.label} ${s.id}`.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );

  const CHECK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
  const CHEV = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>';
  const SEARCH = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>';
</script>

<div class="filter">
  <span class="search-icon">{@html SEARCH}</span>
  <input bind:value={query} placeholder="Cari sumber" aria-label="Cari sumber" />
</div>

<div class="list">
  {#each filtered as s (s.id)}
    {@const selected = active === s.id}
    {@const tint = tagColor(s.id, themeStore.darkMode)}
    <button class="row" class:selected onclick={() => onSelect(s.id)}>
      <span class="tile" style:--tint={tint}>
        {#if s.iconUrl && !iconFailed[s.id]}
          <img
            src={s.iconUrl}
            alt=""
            loading="lazy"
            referrerpolicy="no-referrer"
            onerror={() => (iconFailed[s.id] = true)}
          />
        {:else}
          {s.label.slice(0, 2).toUpperCase()}
        {/if}
      </span>
      <span class="info">
        <strong class:selected>{s.label}</strong>
        <small>{s.id} • {s.version}</small>
        <small class="hint">{selected ? "Sedang dipilih" : "Ketuk untuk mengganti"}</small>
      </span>
      {#if selected}
        <span class="check">{@html CHECK}</span>
      {:else}
        <span class="chev">{@html CHEV}</span>
      {/if}
    </button>
  {:else}
    <p class="empty">Tidak ada sumber cocok “{query}”.</p>
  {/each}
</div>

<style>
  .filter {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid var(--input);
    border-radius: var(--radius);
    background: var(--muted);
    padding: 0 12px;
    margin-bottom: 12px;
  }
  .search-icon {
    display: inline-flex;
    color: var(--muted-foreground);
  }
  .search-icon :global(svg) {
    width: 18px;
    height: 18px;
  }
  .filter input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 10px 0;
    outline: none;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--card);
    cursor: pointer;
    text-align: left;
  }
  .row:hover {
    border-color: var(--primary);
  }
  .row.selected {
    border-color: var(--primary);
  }
  .tile {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: color-mix(in srgb, var(--tint) 22%, var(--card));
    border: 1px solid color-mix(in srgb, var(--tint) 45%, transparent);
    color: var(--tint);
    font-weight: 800;
    font-size: 15px;
    overflow: hidden;
  }
  .tile img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .info strong {
    font-size: 15px;
  }
  .info strong.selected {
    color: var(--primary);
  }
  .info small {
    font-size: 12px;
    color: var(--muted-foreground);
  }
  .check {
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--primary);
    color: var(--primary-foreground);
  }
  .check :global(svg) {
    width: 15px;
    height: 15px;
  }
  .chev {
    display: inline-flex;
    color: var(--muted-foreground);
    opacity: 0.6;
  }
  .chev :global(svg) {
    width: 16px;
    height: 16px;
  }
  .empty {
    color: var(--muted-foreground);
    font-size: 14px;
    text-align: center;
    padding: 16px 0;
  }
</style>
