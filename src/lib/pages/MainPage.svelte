<script lang="ts">
  // MainPage — wireframe gaming-store-catalog diterjemah ke Svelte:
  // sidebar 256 + header 64 sticky + tabs 44 + konten max 1180.
  // Styling 100% token Kuron (bukan warna wireframe).
  import { contentStore } from "../stores/content.svelte";
  import { helloStore } from "../stores/hello.svelte";
  import { SOURCES, sourceStore } from "../stores/source.svelte";
  import { openAboutWindow } from "../api/window";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Sidebar from "../components/Sidebar.svelte";
  import ThemeToggle from "../components/ThemeToggle.svelte";
  import MainFeaturedCard from "../components/MainFeaturedCard.svelte";
  import MainGridCard from "../components/MainGridCard.svelte";

  const ICONS = {
    home: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 10.5 12 3l9 7.5"/><path d="M5 9.5V21h14V9.5"/></svg>',
    download: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M4 21h16"/></svg>',
    bolt: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/></svg>',
    history: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/><path d="M12 7v5l3 3"/></svg>',
    heart: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20.5C7 16.5 3 13.2 3 9.3 3 6.4 5.2 4.5 7.7 4.5c1.7 0 3.3.9 4.3 2.4 1-1.5 2.6-2.4 4.3-2.4 2.5 0 4.7 1.9 4.7 4.8 0 3.9-4 7.2-9 11.2Z"/></svg>',
    settings: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2Z"/></svg>',
    info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>',
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
        { id: "about", label: "Tentang", icon: ICONS.info },
      ],
    },
  ];

  let activeNav = $state("home");
  let collapsed = $state(false);
  let name = $state("");
  let navError = $state<string | null>(null);
  let featured = $derived(contentStore.feed[0] ?? null);
  let rest = $derived(contentStore.feed.slice(1));
  let version = $derived(helloStore.info?.version ?? "0.1.0");

  function submit(e: SubmitEvent) {
    e.preventDefault();
    helloStore.sayHello(name);
  }

  // "Tentang" buka popup window native (bukan ganti konten main).
  async function selectNav(id: string) {
    if (id === "about") {
      navError = await openAboutWindow();
      return;
    }
    navError = null;
    activeNav = id;
  }

  onMount(() => {
    listen("toggle-sidebar", () => (collapsed = !collapsed)).catch(() => {
      // mode browser: event Tauri tidak ada
    });
    listen("reload-page", () => window.location.reload()).catch(() => {
      // mode browser: event Tauri tidak ada
    });
  });
</script>

<div class="shell">
  <Sidebar
    groups={GROUPS}
    active={activeNav}
    {collapsed}
    source={sourceStore.current}
    {version}
    onSelect={selectNav}
    onToggle={() => (collapsed = !collapsed)}
  />

  <div class="main-col">
    <header class="top-header">
      <button class="icon-btn" onclick={() => (collapsed = !collapsed)} title="Toggle sidebar">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6h16M4 12h16M4 18h16"/></svg>
      </button>
      <input class="search" placeholder="Cari judul, tag, artis… (Fase 4)" disabled />
      <div class="head-end">
        <ThemeToggle />
      </div>
    </header>

    <div class="tabs" role="tablist" aria-label="Sumber">
      {#each SOURCES as source}
        <button
          role="tab"
          aria-selected={sourceStore.current === source}
          class:active={sourceStore.current === source}
          onclick={() => sourceStore.select(source)}
        >
          {source}
        </button>
      {/each}
    </div>

    <main class="content">
      {#if navError}
        <p class="err">{navError}</p>
      {/if}
      {#if contentStore.loading && contentStore.feed.length === 0}
        <p class="muted">Memuat feed…</p>
      {/if}
      {#if contentStore.error}
        <p class="err">{contentStore.error}</p>
      {/if}

      {#if featured}
        <MainFeaturedCard content={featured} />
      {/if}

      <section>
        <h2>Terbaru</h2>
        <div class="grid">
          {#each rest as item (item.id)}
            <MainGridCard content={item} />
          {/each}
        </div>
      </section>

      <section class="card">
        <h2>Backend status</h2>
        <p class="muted">
          {helloStore.info
            ? `${helloStore.info.name} v${helloStore.info.version} · ${helloStore.info.backend}`
            : "loading backend info…"}
        </p>
        <form class="row" onsubmit={submit}>
          <input bind:value={name} placeholder="Test IPC sapa…" />
          <button type="submit">Sapa</button>
        </form>
        {#if helloStore.hello}
          <p class="msg">{helloStore.hello.message}</p>
        {/if}
      </section>
    </main>
  </div>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
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
  .search {
    flex: 1;
    max-width: 420px;
    padding: 8px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--input);
    background: var(--muted);
  }
  .tabs {
    height: 44px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 24px;
    background: var(--popover);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .tabs button {
    padding: 6px 14px;
    border-radius: 999px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted-foreground);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }
  .tabs button:hover {
    color: var(--foreground);
    background: var(--muted);
  }
  .tabs button.active {
    color: var(--primary);
    border-color: var(--primary);
    background: color-mix(in srgb, var(--primary) 12%, transparent);
    font-weight: 600;
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
  .card {
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 20px;
    background: var(--card);
  }
  .muted {
    color: var(--muted-foreground);
    font-size: 14px;
  }
  .err {
    color: var(--destructive);
  }
  .msg {
    color: var(--primary);
    font-weight: 600;
  }
  .row {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .row input {
    flex: 1;
    padding: 8px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--input);
    background: var(--muted);
  }
  button[type="submit"] {
    padding: 8px 20px;
    border-radius: var(--radius);
    border: none;
    background: var(--primary);
    color: var(--primary-foreground);
    font-weight: 600;
    cursor: pointer;
  }
</style>
