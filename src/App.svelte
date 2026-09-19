<script lang="ts">
  // AppShell — splash -> main (ganti GoRouter, spec §16).
  import { routeStore } from "./lib/router/route.svelte";
  import SplashScreen from "./lib/pages/SplashScreen.svelte";
  import MainPage from "./lib/pages/MainPage.svelte";
</script>

{#if routeStore.current === "splash"}
  <SplashScreen />
{:else}
  <MainPage />
{/if}
