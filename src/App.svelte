<script lang="ts">
  // AppShell — splash -> main (ganti GoRouter, spec §16).
  // Hash #source-picker = popup window native pilih sumber.
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { routeStore } from "./lib/router/route.svelte";
  import { sourceStore } from "./lib/stores/source.svelte";
  import SplashScreen from "./lib/pages/SplashScreen.svelte";
  import MainPage from "./lib/pages/MainPage.svelte";
  import SourcePickerPage from "./lib/pages/SourcePickerPage.svelte";

  const isPicker = window.location.hash === "#source-picker";

  onMount(() => {
    if (isPicker) return;
    listen<string>("source-selected", (e) => sourceStore.select(e.payload)).catch(() => {
      // mode browser: event Tauri tidak ada
    });
  });
</script>

{#if isPicker}
  <SourcePickerPage />
{:else if routeStore.current === "splash"}
  <SplashScreen />
{:else}
  <MainPage />
{/if}
