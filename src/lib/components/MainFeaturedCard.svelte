<script lang="ts">
  // MainFeaturedCard — port `main_featured_card.dart`.
  import type { Content } from "../domain/types";
  import { tagColor } from "../theme/tokens";
  import { themeStore } from "../stores/theme.svelte";
  import TagChip from "./TagChip.svelte";

  let { content }: { content: Content } = $props();
  let tint = $derived(tagColor(content.id, themeStore.darkMode));
</script>

<article class="featured">
  <div>
    <p class="kicker">Featured</p>
    <h2>{content.title}</h2>
    <p class="meta">Sumber {content.source_id} · ID {content.id} · mock Fase 0</p>
    <TagChip label={content.source_id} type="language" />
  </div>
  <div class="cover" style:--tint={tint} aria-hidden="true">
    <span>{content.title.slice(0, 2).toUpperCase()}</span>
  </div>
</article>

<style>
  .featured {
    display: flex;
    gap: 16px;
    align-items: center;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 20px;
  }
  .featured > div:first-child {
    flex: 1;
  }
  .kicker {
    color: var(--primary);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin: 0 0 8px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 22px;
  }
  .meta {
    margin: 0 0 12px;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  .cover {
    width: 120px;
    height: 160px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--tint) 22%, var(--card));
    border: 1px solid color-mix(in srgb, var(--tint) 45%, transparent);
    font-size: 40px;
    font-weight: 800;
    color: var(--tint);
  }
</style>
