<script lang="ts">
  // MainPage — port `main_screen_scrollable.dart` (ringkas Fase 0):
  // header + featured + grid + backend status. Search/detail/reader Fase 4-5.
  import logo from "../../assets/icons/logo_app.webp";
  import { contentStore } from "../stores/content.svelte";
  import { helloStore } from "../stores/hello.svelte";
  import { themeStore, MODES } from "../stores/theme.svelte";
  import MainFeaturedCard from "../components/MainFeaturedCard.svelte";
  import MainGridCard from "../components/MainGridCard.svelte";

  let name = $state("");
  let featured = $derived(contentStore.feed[0] ?? null);
  let rest = $derived(contentStore.feed.slice(1));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    helloStore.sayHello(name);
  }
</script>

<header class="topbar">
  <div class="brand">
    <img src={logo} alt="Kuron" width="28" height="28" />
    <strong>Kuron</strong>
  </div>
  <div class="modes" role="group" aria-label="Tema">
    {#each MODES as mode}
      <button
        class:active={themeStore.mode === mode}
        onclick={() => themeStore.set(mode)}
      >
        {mode}
      </button>
    {/each}
  </div>
</header>

<main class="container">
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

<style>
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: var(--popover);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 10;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brand img {
    border-radius: 6px;
  }
  .modes {
    display: flex;
    gap: 8px;
  }
  .modes button {
    padding: 4px 12px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
  }
  .modes button.active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
    font-weight: 600;
  }
  .container {
    max-width: 960px;
    margin: 0 auto;
    padding: 24px 16px 48px;
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
  input {
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
