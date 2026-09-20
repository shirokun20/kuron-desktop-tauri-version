<script lang="ts">
  // MainFeaturedCard — banner putar 5 terbaru (saran: ganti featured statis).
  // Backdrop cover blur + panel info + dots; auto 6 dtk, jeda saat hover.
  import { onMount } from "svelte";
  import type { Content } from "../domain/types";
  import { langFlag, langLabel } from "../utils/lang";

  let { items }: { items: Content[] } = $props();

  let index = $state(0);
  let hovering = $state(false);
  let imgFailed = $state<Record<string, boolean>>({});
  let timer: ReturnType<typeof setInterval> | null = null;

  const shown = $derived(items.slice(0, 5));
  const current = $derived(shown.length > 0 ? shown[index % shown.length] : null);

  const BOOK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>';

  onMount(() => {
    timer = setInterval(() => {
      if (!hovering && shown.length > 1) index = (index + 1) % shown.length;
    }, 6000);
    return () => {
      if (timer) clearInterval(timer);
    };
  });
</script>

{#if current}
  <section
    class="banner"
    aria-label="Sorotan terbaru"
    onmouseenter={() => (hovering = true)}
    onmouseleave={() => (hovering = false)}
  >
    {#if current.cover_url && !imgFailed[current.id]}
      <img
        class="backdrop"
        src={current.cover_url}
        alt=""
        aria-hidden="true"
        loading="eager"
        draggable="false"
        referrerpolicy="no-referrer"
        onerror={() => (imgFailed[current.id] = true)}
      />
    {/if}
    <div class="veil"></div>
    <div class="body">
      {#if current.cover_url && !imgFailed[current.id]}
        <img
          class="thumb"
          src={current.cover_url}
          alt={current.title}
          loading="eager"
          draggable="false"
          referrerpolicy="no-referrer"
          onerror={() => (imgFailed[current.id] = true)}
        />
      {/if}
      <div class="info">
        <p class="kicker">Terbaru · {index + 1}/{shown.length}</p>
        <h2>{current.title}</h2>
        <p class="meta">
          {#if current.language}{langFlag(current.language)} {langLabel(current.language)} · {/if}{current.source_id}
          {#if current.page_count} · {current.page_count} halaman{/if}
        </p>
        <div class="dots" role="tablist" aria-label="Pilih sorotan">
          {#each shown as s, i}
            <button
              role="tab"
              aria-selected={i === index % shown.length}
              aria-label={s.title}
              class:active={i === index % shown.length}
              onclick={() => (index = i)}
            ></button>
          {/each}
        </div>
      </div>
      <span class="badge pages"><span class="ic">{@html BOOK}</span>{current.page_count ?? "—"}</span>
    </div>
  </section>
{/if}

<style>
  .banner {
    position: relative;
    border: 1px solid var(--border);
    border-radius: 20px;
    overflow: hidden;
    min-height: 260px;
    background: var(--card);
  }
  .backdrop {
    position: absolute;
    inset: -20px;
    width: calc(100% + 40px);
    height: calc(100% + 40px);
    object-fit: cover;
    filter: blur(18px) brightness(0.55);
  }
  .veil {
    position: absolute;
    inset: 0;
    background: linear-gradient(to right, rgba(0, 0, 0, 0.72) 20%, rgba(0, 0, 0, 0.25));
  }
  .body {
    position: relative;
    display: flex;
    gap: 20px;
    align-items: center;
    padding: 28px;
  }
  .thumb {
    width: 130px;
    aspect-ratio: 3 / 4;
    object-fit: cover;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.25);
    flex-shrink: 0;
  }
  .info {
    flex: 1;
    min-width: 0;
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
    color: #fff;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .meta {
    margin: 0 0 14px;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.75);
  }
  .dots {
    display: flex;
    gap: 8px;
  }
  .dots button {
    width: 24px;
    height: 6px;
    border-radius: 999px;
    border: none;
    background: rgba(255, 255, 255, 0.3);
    cursor: pointer;
    padding: 0;
  }
  .dots button.active {
    background: var(--primary);
  }
  .badge.pages {
    position: absolute;
    top: 20px;
    right: 20px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 700;
    padding: 5px 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--primary) 85%, transparent);
    color: #fff;
  }
  .ic {
    display: inline-flex;
  }
  .ic :global(svg) {
    width: 13px;
    height: 13px;
  }
</style>
