<script lang="ts">
  // MainPage — wireframe gaming-store-catalog diterjemah ke Svelte:
  // sidebar 256 + header 64 sticky + konten max 1180. Sumber via sidebar.
  // Styling 100% token Kuron (bukan warna wireframe).
  import { contentStore } from "../stores/content.svelte";
  import { helloStore } from "../stores/hello.svelte";
  import { libraryStore } from "../stores/library.svelte";
  import { sourceStore } from "../stores/source.svelte";
  import { clearSavedQuery, loadSavedQuery } from "../stores/filterPersist";
  import { overlayStore } from "../stores/overlay.svelte";
  import { platformStore } from "../stores/platform.svelte";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Sidebar from "../components/Sidebar.svelte";
  import ThemeToggle from "../components/ThemeToggle.svelte";
  import MainFeaturedCard from "../components/MainFeaturedCard.svelte";
  import MainGridCard from "../components/MainGridCard.svelte";
  import LibraryPage from "./LibraryPage.svelte";
  import { routeStore } from "../router/route.svelte";
  import type { Content } from "../domain/types";

  const ICONS = {
    home: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 10.5 12 3l9 7.5"/><path d="M5 9.5V21h14V9.5"/></svg>',
    download: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M4 21h16"/></svg>',
    bolt: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/></svg>',
    history: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/><path d="M12 7v5l3 3"/></svg>',
    heart: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20.5C7 16.5 3 13.2 3 9.3 3 6.4 5.2 4.5 7.7 4.5c1.7 0 3.3.9 4.3 2.4 1-1.5 2.6-2.4 4.3-2.4 2.5 0 4.7 1.9 4.7 4.8 0 3.9-4 7.2-9 11.2Z"/></svg>',
    settings: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2Z"/></svg>',
    info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>',
    ext: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 7V5a2 2 0 0 0-4 0v2H7a2 2 0 0 0-2 2v3h2a2 2 0 1 1 0 4H5v3a2 2 0 0 0 2 2h3v-2a2 2 0 1 1 4 0v2h3a2 2 0 0 0 2-2v-3h-2a2 2 0 1 1 0-4h2V9a2 2 0 0 0-2-2h-3Z"/></svg>',
  };

  const GROUPS = [
    {
      label: "BERANDA",
      items: [
        { id: "home", label: "Beranda", icon: ICONS.home },
        { id: "downloads", label: "Galeri yang diunduh", icon: ICONS.download },
        { id: "offline", label: "Konten Offline", icon: ICONS.bolt },
      ],
    },
    {
      label: "EXPLORE",
      items: [
        { id: "favorites", label: "Galeri favorit", icon: ICONS.heart },
        { id: "history", label: "Lihat riwayat", icon: ICONS.history },
      ],
    },
    {
      label: "MORE",
      items: [
        { id: "settings", label: "Pengaturan", icon: ICONS.settings },
        { id: "extensions", label: "Ekstensi", icon: ICONS.ext },
        { id: "about", label: "Tentang", icon: ICONS.info },
      ],
    },
  ];

  let activeNav = $state("home");
  // Rantai baca (kartu → detail → reader) pindah ke route khusus fullscreen
  // (routeStore) — MainPage murni beranda + library.
  // Desktop: `collapsed` = sidebar mini. Mobile (layar sempit): `drawerOpen`.
  let collapsed = $state(false);
  let drawerOpen = $state(false);
  let version = $derived(helloStore.info?.version ?? "0.1.0");

  /** Hamburger: mini-sidebar di layar lebar, drawer di layar sempit. */
  function toggleSidebar() {
    if (platformStore.narrow) drawerOpen = !drawerOpen;
    else collapsed = !collapsed;
  }

  // SATU pintu tap untuk Sumber/Filter/Tentang/Ekstensi: `overlayStore` yang
  // memilih popup window native (desktop Tauri layar lebar) atau overlay
  // in-app (web & mobile) — lihat `stores/overlay.svelte`.
  async function openFilter() {
    await overlayStore.open("filter");
  }

  // "Tentang"/"Ekstensi" muncul sebagai overlay/popup, bukan ganti panel nav.
  async function selectNav(id: string) {
    drawerOpen = false;
    if (id === "about" || id === "extensions") {
      await overlayStore.open(id);
      return;
    }
    activeNav = id;
  }

  /** Kartu/baris diklik (feed, favorit, riwayat) → route detail khusus. */
  function selectContent(c: Content) {
    routeStore.openDetail(c);
  }

  onMount(() => {
    // Favorit untuk tombol hati di kartu (gagal → kosong, bukan error merah).
    libraryStore.load();
    listen("toggle-sidebar", () => toggleSidebar()).catch(() => {
      // mode browser: event Tauri tidak ada
    });
    listen("reload-page", () => window.location.reload()).catch(() => {
      // mode browser: event Tauri tidak ada
    });
  });

  // Overlay apa pun (mis. "Pilih Sumber" dari sidebar) menutup drawer mobile.
  $effect(() => {
    if (overlayStore.kind) drawerOpen = false;
  });

  // Feed ikut sumber aktif (termasuk hasil install ekstensi).
  // Sumber yang punya pencarian tersimpan (`kuron.filter.<source>`) langsung
  // dibuka dalam mode itu — termasuk saat aplikasi baru dijalankan / window
  // baru; tanpa simpanan → beranda.
  $effect(() => {
    const s = sourceStore.current;
    const saved = loadSavedQuery(s);
    if (saved) contentStore.search(s, saved.query, saved.label);
    else contentStore.load(s);
  });

  // "✕ Bersihkan": keluar dari mode pencarian DAN buang query tersimpannya
  // (nilai form tetap tersimpan) — supaya pencarian tak muncul lagi saat app
  // dibuka ulang atau sumber ditukar-balikkan.
  function clearSearch() {
    clearSavedQuery(sourceStore.current);
    contentStore.clearSearch();
  }
</script>

<div class="shell" class:narrow={platformStore.narrow}>
  {#if platformStore.narrow && drawerOpen}
    <button
      class="drawer-backdrop"
      type="button"
      onclick={() => (drawerOpen = false)}
      aria-label="Tutup menu"
    ></button>
  {/if}
  <div class="sidebar-slot" class:hidden-narrow={platformStore.narrow && !drawerOpen}>
    <Sidebar
      groups={GROUPS}
      active={activeNav}
      collapsed={platformStore.narrow ? false : collapsed}
      source={sourceStore.currentLabel}
      sourceIcon={sourceStore.currentIconUrl}
      {version}
      onSelect={selectNav}
      onToggle={toggleSidebar}
    />
  </div>

  <div class="main-col">
    <header class="top-header">
      <button class="icon-btn" onclick={toggleSidebar} title="Toggle sidebar">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6h16M4 12h16M4 18h16"/></svg>
      </button>
      <button class="icon-btn filter-btn" onclick={openFilter} title="Cari di {sourceStore.currentLabel}">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>
        {#if contentStore.searchMode === "filter"}<span class="dot"></span>{/if}
      </button>
      <div class="head-end">
        <ThemeToggle />
      </div>
    </header>

    <main class="content">
      {#if overlayStore.error}
        <p class="err">{overlayStore.error}</p>
      {/if}
      {#if activeNav === "favorites" || activeNav === "history"}
        <LibraryPage tab={activeNav} onselect={selectContent} />
      {:else}
      {#if contentStore.loading && contentStore.feed.length === 0}
        <p class="muted">Memuat feed…</p>
      {/if}
      {#if contentStore.error}
        <p class="err">{contentStore.error}</p>
      {/if}

      {#if contentStore.offline}
        <!-- Mode web (browser): backend Rust tidak bisa dijangkau dari tab -->
        <!-- mana pun. Kondisi rapi + pintu ke overlay, bukan banner error. -->
        <section class="empty offline">
          <h2>Mode Web — tanpa backend</h2>
          <p>
            UI berjalan penuh, tapi feed &amp; pencarian butuh backend Rust yang
            hanya hidup di dalam aplikasi desktop (Tauri). Tab browser tidak
            bisa memanggilnya, jadi data nyata tidak tersedia di sini.
          </p>
          <p class="muted">
            Untuk feed nyata: jalankan aplikasi desktop
            (<code>pnpm dev</code> atau binary hasil <code>pnpm build</code>).
          </p>
          <div class="actions">
            <button class="ghost" onclick={() => overlayStore.open("source")}>
              Pilih Sumber
            </button>
            <button
              class="ghost"
              onclick={() => contentStore.load(sourceStore.current)}
            >
              Coba Muat Ulang
            </button>
          </div>
        </section>
      {/if}

      {#if contentStore.searchQuery && !contentStore.offline}
        <section class="searchbar">
          <strong>{contentStore.searchMode === "filter" ? "Filter" : "Hasil Pencarian"}</strong>
          <span class="query">
            {contentStore.searchMode === "filter" ? "Kriteria" : "Kueri"}: "{contentStore.searchLabel}"
            · {contentStore.feed.length} hasil
          </span>
          <button class="ghost" onclick={clearSearch}>
            ✕ Bersihkan
          </button>
        </section>
      {/if}

      {#if !contentStore.searchQuery && contentStore.feed.length > 0}
        <MainFeaturedCard items={contentStore.feed} />
      {/if}

      {#if !contentStore.loading && !contentStore.searching && contentStore.feed.length === 0 && (contentStore.searchQuery || (!contentStore.error && !contentStore.offline))}
        <section class="empty">
          {#if contentStore.searchQuery}
            <p>Tidak ada hasil untuk "{contentStore.searchLabel}" di {sourceStore.currentLabel}.</p>
            <p class="muted">Coba kata kunci lain — atau pakai Filter untuk bahasa, tag, dan status.</p>
          {:else}
            <p class="muted">Feed kosong — coba Muat ulang atau ganti sumber.</p>
          {/if}
        </section>
      {/if}

      {#if contentStore.feed.length > 0}
      <section>
        <h2>{contentStore.searchQuery ? `Hasil (${contentStore.feed.length})` : "Terbaru"}</h2>
        <div class="grid">
          {#each contentStore.feed as item (item.id)}
            <MainGridCard content={item} onselect={selectContent} />
          {/each}
        </div>
        <div class="pager">
          {#if contentStore.hasMore}
            <button
              class="more"
              onclick={() => contentStore.loadMore()}
              disabled={contentStore.loadingMore}
            >
              {contentStore.loadingMore ? "Memuat…" : "Muat lebih banyak"}
            </button>
          {:else if contentStore.feed.length > 0}
            <p class="muted">Semua sudah dimuat.</p>
          {/if}
        </div>
      </section>
      {/if}
      {/if}
    </main>
  </div>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .sidebar-slot {
    display: flex;
    flex-shrink: 0;
  }
  .hidden-narrow {
    display: none;
  }
  /* Mobile (layar sempit): sidebar jadi drawer di atas konten. */
  .shell.narrow .sidebar-slot {
    position: fixed;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 60;
    box-shadow: 0 0 40px rgba(0, 0, 0, 0.45);
  }
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    border: 0;
    padding: 0;
    background: rgba(0, 0, 0, 0.55);
    cursor: default;
  }
  .main-col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }
  .top-header {
    height: 64px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 24px;
    background: var(--popover);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 10;
  }
  .head-end {
    margin-left: auto;
    display: flex;
    align-items: center;
  }
  .icon-btn {
    display: inline-flex;
    padding: 8px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--muted-foreground);
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--muted);
    color: var(--foreground);
  }
  .icon-btn svg {
    width: 20px;
    height: 20px;
  }
  .filter-btn {
    position: relative;
  }
  .filter-btn .dot {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--primary);
  }
  .searchbar {
    display: flex;
    align-items: center;
    gap: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 10px 16px;
    background: var(--card);
  }
  .query {
    flex: 1;
    color: var(--muted-foreground);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ghost {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 6px 14px;
    cursor: pointer;
    color: var(--primary);
    font-weight: 600;
  }
  .empty {
    border: 1px dashed var(--border);
    border-radius: 12px;
    padding: 24px;
    text-align: center;
  }
  /* Kondisi mode web (tanpa backend Rust): panel penjelasan, bukan error. */
  .offline h2 {
    margin: 0 0 8px;
  }
  .offline code {
    background: var(--muted);
    border-radius: 6px;
    padding: 2px 6px;
    font-size: 12px;
  }
  .offline .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: center;
    margin-top: 16px;
  }
  .content {
    max-width: 1180px;
    width: 100%;
    margin: 0 auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 18px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }
  .pager {
    display: flex;
    justify-content: center;
    padding: 20px 0 8px;
  }
  .more {
    padding: 10px 28px;
    border-radius: 999px;
    border: 1px solid var(--primary);
    background: transparent;
    color: var(--primary);
    font-weight: 700;
    font-size: 14px;
    cursor: pointer;
  }
  .more:disabled {
    opacity: 0.6;
    cursor: wait;
  }
  .more:hover:not(:disabled) {
    background: color-mix(in srgb, var(--primary) 12%, transparent);
  }
  .muted {
    color: var(--muted-foreground);
    font-size: 14px;
  }
  .err {
    color: var(--destructive);
  }
</style>
