<script lang="ts">
  // MainGridCard — port `main_grid_card.dart`: flat + bordered (elevation 0).
  // Cover duotone deterministik dari id (tanpa gambar asli sebelum Fase 2).
  import type { Content } from "../domain/types";
  import { tagColor } from "../theme/tokens";
  import { themeStore } from "../stores/theme.svelte";
  import TagChip from "./TagChip.svelte";

  let { content }: { content: Content } = $props();
  let tint = $derived(tagColor(content.id, themeStore.darkMode));
</script>

<article class="grid-card">
  <div class="cover" style:--tint={tint} aria-hidden="true">
    <span class="cover-title">{content.title.slice(0, 2).toUpperCase()}</span>
  </div>
  <h3>{content.title}</h3>
  <TagChip label={content.source_id} type="group" />
</article>

<style>
  .grid-card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 12px;
    transition: border-color 150ms ease;
  }
  .grid-card:hover {
    border-color: var(--primary);
  }
  .cover {
    aspect-ratio: 3 / 4;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--tint) 22%, var(--card));
    border: 1px solid color-mix(in srgb, var(--tint) 45%, transparent);
    margin-bottom: 8px;
  }
  .cover-title {
    font-size: 32px;
    font-weight: 800;
    color: var(--tint);
  }
  h3 {
    margin: 0 0 8px;
    font-size: 14px;
    font-weight: 600;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
