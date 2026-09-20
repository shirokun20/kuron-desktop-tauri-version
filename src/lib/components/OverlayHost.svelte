<script lang="ts">
  // OverlayHost — rumah TUNGGAL isi overlay in-app (web & mobile). Di desktop
  // Tauri layar lebar isi ini muncul sebagai popup window native, jadi
  // halaman-halamannya dipakai ulang apa adanya (satu komponen, dua tempat).
  import { overlayStore } from "../stores/overlay.svelte";
  import { platformStore } from "../stores/platform.svelte";
  import { contentStore } from "../stores/content.svelte";
  import { sourceStore } from "../stores/source.svelte";
  import SourcePickerPage from "../pages/SourcePickerPage.svelte";
  import AboutPage from "../pages/AboutPage.svelte";
  import ExtensionManagerPage from "../pages/ExtensionManagerPage.svelte";
  import FilterPage from "../pages/FilterPage.svelte";

  const TITLES: Record<string, string> = {
    source: "Pilih Sumber",
    filter: "Filter Pencarian",
    about: "Tentang Kuron",
    extensions: "Ekstensi",
  };

  let kind = $derived(overlayStore.kind);
  let title = $derived(kind ? (TITLES[kind] ?? "Kuron") : "");
  // Filter punya header + tombol ✕ sendiri; yang lain pakai header host.
  let ownHeader = $derived(kind === "filter");

  async function onFilterSubmit(payload: { query: string; label: string }) {
    await contentStore.search(sourceStore.current, payload.query, payload.label);
    overlayStore.close();
  }
</script>

{#if kind}
  <div
    class="overlay"
    class:narrow={platformStore.narrow}
    role="dialog"
    aria-modal="true"
    aria-label={title}
  >
    <button
      class="backdrop"
      type="button"
      onclick={() => overlayStore.close()}
      aria-label="Tutup {title}"
    ></button>
    <section class="panel">
      {#if !ownHeader}
        <header class="panel-head">
          <strong>{title}</strong>
          <button
            class="close"
            type="button"
            onclick={() => overlayStore.close()}
            aria-label="Tutup {title}"
          >
            ✕
          </button>
        </header>
      {/if}
      <div class="panel-body">
        {#if kind === "source"}
          <SourcePickerPage onClose={() => overlayStore.close()} />
        {:else if kind === "filter"}
          <FilterPage onClose={() => overlayStore.close()} onSubmit={onFilterSubmit} />
        {:else if kind === "about"}
          <AboutPage />
        {:else if kind === "extensions"}
          <ExtensionManagerPage />
        {/if}
      </div>
    </section>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    border: 0;
    padding: 0;
    background: rgba(0, 0, 0, 0.55);
    cursor: default;
  }
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    width: min(760px, 100%);
    max-height: min(92vh, 820px);
    border: 1px solid var(--border);
    border-bottom: 0;
    border-radius: 18px 18px 0 0;
    background: var(--background);
    box-shadow: 0 -12px 40px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }
  .panel-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px 12px 18px;
    border-bottom: 1px solid var(--border);
  }
  .panel-head strong {
    flex: 1;
    font-size: 15px;
  }
  .close {
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--muted-foreground);
    cursor: pointer;
    font-size: 14px;
    padding: 6px 10px;
  }
  .close:hover {
    color: var(--foreground);
    background: color-mix(in srgb, var(--foreground) 10%, transparent);
  }
  .panel-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  /* Halaman isi dibuat untuk halaman penuh → jangan paksa 100vh di sheet. */
  .panel-body > :global(*) {
    min-height: 0;
  }
  /* Mobile: full-screen (bukan sheet). */
  .overlay.narrow {
    align-items: stretch;
  }
  .overlay.narrow .panel {
    width: 100%;
    height: 100%;
    max-height: 100%;
    border: 0;
    border-radius: 0;
  }
</style>
