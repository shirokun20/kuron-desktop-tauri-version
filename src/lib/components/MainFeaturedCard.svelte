<script lang="ts">
  // MainFeaturedCard — port `main_featured_card.dart`.
  import type { Content } from "../domain/types";
  import TagChip from "./TagChip.svelte";

  let { content }: { content: Content } = $props();
</script>

<article class="featured">
  <div>
    <p class="kicker">Featured</p>
    <h2>{content.title}</h2>
    <TagChip label={content.source_id} type="language" />
  </div>
  <div class="cover" aria-hidden="true">
    <span>{content.title.slice(0, 2).toUpperCase()}</span>
  </div>
</article>

<style>
  .featured {
    display: flex;
    gap: 16px;
    align-items: center;
    background: linear-gradient(var(--kuron-gradient-start), var(--kuron-gradient-end));
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
    margin: 0 0 12px;
    font-size: 22px;
  }
  .cover {
    width: 120px;
    height: 160px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--muted);
    border: 1px solid var(--border);
    font-size: 40px;
    font-weight: 800;
    color: var(--primary);
  }
</style>
