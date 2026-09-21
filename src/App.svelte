<script lang="ts">
  // AppShell — splash -> main (ganti GoRouter, spec §16).
  // Hash #source-picker / #about / #extensions / #filter = popup window native
  // (desktop Tauri); di web & mobile isi yang sama muncul sebagai overlay
  // in-app lewat `<OverlayHost />` (lihat `stores/overlay.svelte`).
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { routeStore } from "./lib/router/route.svelte";
  import { platformStore } from "./lib/stores/platform.svelte";
  import { sourceStore } from "./lib/stores/source.svelte";
  import { contentStore } from "./lib/stores/content.svelte";
  import SplashScreen from "./lib/pages/SplashScreen.svelte";
  import MainPage from "./lib/pages/MainPage.svelte";
  import DetailPage from "./lib/pages/DetailPage.svelte";
  import ReaderPage from "./lib/pages/ReaderPage.svelte";
  import SourcePickerPage from "./lib/pages/SourcePickerPage.svelte";
  import AboutPage from "./lib/pages/AboutPage.svelte";
  import ExtensionManagerPage from "./lib/pages/ExtensionManagerPage.svelte";
  import FilterPage from "./lib/pages/FilterPage.svelte";
  import OverlayHost from "./lib/components/OverlayHost.svelte";

  const hash = window.location.hash;
  const isPicker = hash === "#source-picker";
  const isAbout = hash === "#about";
  const isExt = hash === "#extensions";
  const isFilter = hash === "#filter";

  onMount(() => {
    // Snapshot platform (tauri/web + lebar layar) untuk semua keputusan tap.
    platformStore.init();
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
  <svelte:boundary>
    <FilterPage />
    {#snippet failed(error)}
      <main class="boot-err">
        <h1>Gagal buka Filter</h1>
        <p>{error instanceof Error ? error.message : String(error)}</p>
        <p class="hint">Screenshot teks ini lalu kirim — biar akar masalah ketemu.</p>
      </main>
    {/snippet}
  </svelte:boundary>
{:else if routeStore.current === "splash"}
  <SplashScreen />
{:else if routeStore.current === "detail" && routeStore.detailContent}
  <svelte:boundary>
    <DetailPage
      content={routeStore.detailContent}
      onback={() => routeStore.backToMain()}
      onopenchapter={(ch, list) => routeStore.openReader(ch, list)}
      onselectcontent={(c) => routeStore.openDetail(c)}
    />
    {#snippet failed(error)}
      <main class="boot-err">
        <h1>Gagal buka Detail</h1>
        <p>{error instanceof Error ? error.message : String(error)}</p>
        <p class="hint">Screenshot teks ini lalu kirim — biar akar masalah ketemu.</p>
      </main>
    {/snippet}
  </svelte:boundary>
{:else if routeStore.current === "reader" && routeStore.detailContent && routeStore.readerChapter}
  <svelte:boundary>
    <ReaderPage
      content={routeStore.detailContent}
      chapter={routeStore.readerChapter}
      siblings={routeStore.readerList}
      source={routeStore.detailContent.source_id}
      onback={() => routeStore.backToDetail()}
      onchapter={(ch) => routeStore.openReader(ch, routeStore.readerList)}
    />
    {#snippet failed(error)}
      <main class="boot-err">
        <h1>Gagal buka Reader</h1>
        <p>{error instanceof Error ? error.message : String(error)}</p>
        <p class="hint">Screenshot teks ini lalu kirim — biar akar masalah ketemu.</p>
      </main>
    {/snippet}
  </svelte:boundary>
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

<!-- Overlay in-app (web & mobile): isi identik popup window native desktop. -->
<OverlayHost />

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
