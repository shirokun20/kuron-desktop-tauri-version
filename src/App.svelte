<script lang="ts">
  // AppShell — splash -> main (ganti GoRouter, spec §16).
  // Hash #source-picker / #about / #extensions / #filter = popup window native.
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { routeStore } from "./lib/router/route.svelte";
  import { sourceStore } from "./lib/stores/source.svelte";
  import { contentStore } from "./lib/stores/content.svelte";
  import SplashScreen from "./lib/pages/SplashScreen.svelte";
  import MainPage from "./lib/pages/MainPage.svelte";
  import SourcePickerPage from "./lib/pages/SourcePickerPage.svelte";
  import AboutPage from "./lib/pages/AboutPage.svelte";
  import ExtensionManagerPage from "./lib/pages/ExtensionManagerPage.svelte";
  import FilterPage from "./lib/pages/FilterPage.svelte";

  const hash = window.location.hash;
  const isPicker = hash === "#source-picker";
  const isAbout = hash === "#about";
  const isExt = hash === "#extensions";
  const isFilter = hash === "#filter";

  onMount(() => {
    if (isPicker || isAbout || isExt || isFilter) return;
    listen<string>("source-selected", (e) => sourceStore.select(e.payload)).catch(() => {
      // mode browser: event Tauri tidak ada
    });
    listen<{ query: string; label: string }>("filter-submit", (e) => {
      contentStore.search(sourceStore.current, e.payload.query, e.payload.label);
    }).catch(() => {
      // mode browser: event Tauri tidak ada
    });
  });
</script>

{#if isPicker}
  <SourcePickerPage />
{:else if isAbout}
  <AboutPage />
{:else if isExt}
  <ExtensionManagerPage />
{:else if isFilter}
  <FilterPage />
{:else if routeStore.current === "splash"}
  <SplashScreen />
{:else}
  <svelte:boundary>
    <MainPage />
    {#snippet failed(error)}
      <main class="boot-err">
        <h1>Gagal buka Beranda</h1>
        <p>{error instanceof Error ? error.message : String(error)}</p>
        <p class="hint">Screenshot teks ini lalu kirim — biar akar masalah ketemu.</p>
      </main>
    {/snippet}
  </svelte:boundary>
{/if}

<style>
  .boot-err {
    padding: 40px;
    max-width: 640px;
  }
  .boot-err h1 {
    font-size: 20px;
  }
  .boot-err .hint {
    color: var(--muted-foreground);
    font-size: 13px;
  }
</style>
