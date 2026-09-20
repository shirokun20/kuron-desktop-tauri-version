<script lang="ts">
  // AppShell — splash -> main (ganti GoRouter, spec §16).
  // Hash #source-picker / #about / #extensions = popup window native.
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { routeStore } from "./lib/router/route.svelte";
  import { sourceStore } from "./lib/stores/source.svelte";
  import SplashScreen from "./lib/pages/SplashScreen.svelte";
  import MainPage from "./lib/pages/MainPage.svelte";
  import SourcePickerPage from "./lib/pages/SourcePickerPage.svelte";
  import AboutPage from "./lib/pages/AboutPage.svelte";
  import ExtensionManagerPage from "./lib/pages/ExtensionManagerPage.svelte";

  const hash = window.location.hash;
  const isPicker = hash === "#source-picker";
  const isAbout = hash === "#about";
  const isExt = hash === "#extensions";

  onMount(() => {
    if (isPicker || isAbout || isExt) return;
    listen<string>("source-selected", (e) => sourceStore.select(e.payload)).catch(() => {
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
