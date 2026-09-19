<script lang="ts">
  // MainGridCard — port `main_grid_card.dart`: flat + bordered (elevation 0).
  import type { Content } from "../domain/types";
  import TagChip from "./TagChip.svelte";

  let { content }: { content: Content } = $props();
</script>

<article class="grid-card">
  <div class="cover" aria-hidden="true">
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
  }
  .cover {
    aspect-ratio: 3 / 4;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(var(--kuron-gradient-start), var(--kuron-gradient-end));
    border: 1px solid var(--border);
    margin-bottom: 8px;
  }
  .cover-title {
    font-size: 32px;
    font-weight: 800;
    color: var(--primary);
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
