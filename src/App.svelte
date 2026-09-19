<script lang="ts">
  // AppShell — splash -> main (ganti GoRouter, spec §16).
  // Hash #source-picker / #about = popup window native.
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { routeStore } from "./lib/router/route.svelte";
  import { sourceStore } from "./lib/stores/source.svelte";
  import SplashScreen from "./lib/pages/SplashScreen.svelte";
  import MainPage from "./lib/pages/MainPage.svelte";
  import SourcePickerPage from "./lib/pages/SourcePickerPage.svelte";
  import AboutPage from "./lib/pages/AboutPage.svelte";

  const hash = window.location.hash;
  const isPicker = hash === "#source-picker";
  const isAbout = hash === "#about";

  onMount(() => {
    if (isPicker || isAbout) return;
    listen<string>("source-selected", (e) => sourceStore.select(e.payload)).catch(() => {
      // mode browser: event Tauri tidak ada
    });
  });
</script>

{#if isPicker}
  <SourcePickerPage />
{:else if isAbout}
  <AboutPage />
{:else if routeStore.current === "splash"}
  <SplashScreen />
{:else}
  <MainPage />
{/if}
