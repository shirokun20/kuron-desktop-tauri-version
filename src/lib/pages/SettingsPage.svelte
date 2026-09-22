<script lang="ts">
  // SettingsPage — halaman Pengaturan (9.1, spec `settings-ai`):
  // seksi tampilan (tema + blur thumbnail default-on), pembaca, jaringan,
  // data. Preferensi persist di localStorage (`kuron.settings`).
  import { settingsStore } from "../stores/settings.svelte";
  import { themeStore, MODES } from "../stores/theme.svelte";
  import { platformStore } from "../stores/platform.svelte";
  import { libraryStore } from "../stores/library.svelte";
  import type { ReaderMode } from "../stores/settingsPersist";

  const READER_MODES: { id: ReaderMode; label: string }[] = [
    { id: "paginated", label: "Per halaman" },
    { id: "continuous", label: "Gulir terus" },
    { id: "webtoon", label: "Webtoon" },
  ];

  let runtimeLabel = $derived(
    platformStore.snapshot.runtime === "tauri" ? "Aplikasi desktop" : "Mode web",
  );

  async function clearHistory() {
    if (!confirm("Hapus semua riwayat baca?")) return;
    await libraryStore.clearHistory();
  }

  async function resetAll() {
    if (!confirm("Reset data? Riwayat + favorit + snapshot dihapus.")) return;
    await libraryStore.clearLibrary();
  }
</script>

<section class="settings">
  <header class="set-head halftone">
    <div>
      <p class="kicker">Preferensi</p>
      <h2>Pengaturan</h2>
    </div>
    <span class="mode-badge">{runtimeLabel}</span>
  </header>

  {#if libraryStore.error}
    <p class="err">{libraryStore.error}</p>
  {/if}

  <article class="card">
    <h3>Tampilan</h3>
    <div class="row">
      <div class="txt">
        <strong>Tema</strong>
        <p class="hint">Mode terang / gelap — sama dengan tombol di header.</p>
      </div>
      <div class="seg" role="group" aria-label="Pilih tema">
        {#each MODES as m (m)}
          <button
            type="button"
            class:on={themeStore.mode === m}
            aria-pressed={themeStore.mode === m}
            onclick={() => themeStore.set(m)}
          >
            {m === "dark" ? "Gelap" : "Terang"}
          </button>
        {/each}
      </div>
    </div>
    <div class="row">
      <div class="txt">
        <strong>Blur Thumbnail</strong>
        <p class="hint">
          Kaburkan cover di beranda &amp; riwayat demi privasi. Default aktif
          (ala mobile); matikan agar cover tampil tajam.
        </p>
      </div>
      <button
        type="button"
        class="switch"
        role="switch"
        aria-checked={settingsStore.blurThumbnail}
        aria-label="Blur Thumbnail"
        onclick={() =>
          settingsStore.patch({ blurThumbnail: !settingsStore.blurThumbnail })}
      >
        <span class="knob" class:on={settingsStore.blurThumbnail}></span>
      </button>
    </div>
  </article>

  <article class="card">
    <h3>Pembaca</h3>
    <div class="row">
      <div class="txt">
        <strong>Mode baca</strong>
        <p class="hint">
          Preferensi disimpan untuk reader. Reader kini selalu gulir
          terus-menerus; pilihan lain aktif setelah ReaderCanvas (tugas 7.1).
        </p>
      </div>
      <div class="seg" role="group" aria-label="Mode baca">
        {#each READER_MODES as m (m.id)}
          <button
            type="button"
            class:on={settingsStore.s.readerMode === m.id}
            aria-pressed={settingsStore.s.readerMode === m.id}
            onclick={() => settingsStore.patch({ readerMode: m.id })}
          >
            {m.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row">
      <div class="txt">
        <strong>Kanan ke kiri</strong>
        <p class="hint">
          Arah baca manga RTL. Disimpan kini; diterapkan penuh oleh ReaderCanvas
          (tugas 7.1).
        </p>
      </div>
      <button
        type="button"
        class="switch"
        role="switch"
        aria-checked={settingsStore.s.readerRightToLeft}
        aria-label="Baca kanan ke kiri"
        onclick={() =>
          settingsStore.patch({
            readerRightToLeft: !settingsStore.s.readerRightToLeft,
          })}
      >
        <span
          class="knob"
          class:on={settingsStore.s.readerRightToLeft}
        ></span>
      </button>
    </div>
  </article>

  <article class="card">
    <h3>Jaringan</h3>
    <div class="row">
      <div class="txt">
        <strong>Muat feed otomatis</strong>
        <p class="hint">
          Saat aktif, beranda memuat feed sendiri saat dibuka / sumber diganti.
          Matikan untuk buka app tanpa request — muat manual lewat tombol Muat
          ulang.
        </p>
      </div>
      <button
        type="button"
        class="switch"
        role="switch"
        aria-checked={settingsStore.autoLoadFeed}
        aria-label="Muat feed otomatis"
        onclick={() =>
          settingsStore.patch({ autoLoadFeed: !settingsStore.autoLoadFeed })}
      >
        <span class="knob" class:on={settingsStore.autoLoadFeed}></span>
      </button>
    </div>
    <div class="row">
      <div class="txt">
        <strong>Konteks runtime</strong>
        <p class="hint">
          Data nyata (feed, cari, library) hanya hidup di backend Rust dalam
          aplikasi desktop — tab browser tak memanggilnya.
        </p>
      </div>
      <span class="value">{runtimeLabel}</span>
    </div>
  </article>

  <article class="card">
    <h3>Data</h3>
    <div class="row">
      <div class="txt">
        <strong>Riwayat baca</strong>
        <p class="hint">
          {libraryStore.history.length} entri tersimpan lokal di SQLite
          (<code>kuron.db</code>) — tanpa request keluar.
        </p>
      </div>
      <button
        type="button"
        class="ghost danger"
        disabled={libraryStore.history.length === 0}
        onclick={clearHistory}
      >
        Hapus riwayat
      </button>
    </div>
    <div class="row">
      <div class="txt">
        <strong>Reset semua data</strong>
        <p class="hint">
          {libraryStore.favorites.length} favorit + seluruh riwayat dihapus
          permanen dari perangkat.
        </p>
      </div>
      <button type="button" class="ghost danger" onclick={resetAll}>
        Reset data
      </button>
    </div>
  </article>
</section>

<style>
  .settings {
    --ink-shadow: 4px 4px 0 rgb(0 0 0 / 0.35);
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .halftone {
    background-image: radial-gradient(
      color-mix(in srgb, var(--foreground) 13%, transparent) 1.1px,
      transparent 1.3px
    );
    background-size: 12px 12px;
  }
  .set-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    background-color: var(--card);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 18px 22px;
  }
  .kicker {
    margin: 0 0 2px;
    font-family: "Komika", system-ui, sans-serif;
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .set-head h2 {
    margin: 0;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 34px;
    font-weight: 400;
    letter-spacing: 0.03em;
    line-height: 1;
  }
  .mode-badge {
    font-size: 12px;
    font-weight: 700;
    padding: 5px 12px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
  }
  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 6px 22px;
  }
  .card h3 {
    margin: 0;
    padding: 14px 0 6px;
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 14px 0;
    border-top: 1px solid var(--border);
  }
  .row:first-of-type {
    border-top: 0;
  }
  .txt {
    min-width: 0;
  }
  .txt strong {
    display: block;
    font-size: 14px;
    margin-bottom: 3px;
  }
  .hint {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--muted-foreground);
  }
  .hint code {
    background: var(--muted);
    border-radius: 5px;
    padding: 1px 5px;
    font-size: 11.5px;
  }
  .value {
    flex-shrink: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--foreground);
    background: var(--background);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 5px 12px;
  }
  .seg {
    flex-shrink: 0;
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
    background: var(--background);
  }
  .seg button {
    border: 0;
    background: transparent;
    color: var(--muted-foreground);
    font-size: 13px;
    font-weight: 600;
    padding: 7px 14px;
    cursor: pointer;
  }
  .seg button + button {
    border-left: 1px solid var(--border);
  }
  .seg button.on {
    background: var(--primary);
    color: var(--primary-foreground);
  }
  .switch {
    flex-shrink: 0;
    position: relative;
    width: 46px;
    height: 26px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--muted);
    cursor: pointer;
    padding: 0;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--muted-foreground);
    transition: transform 180ms ease, background 180ms ease;
  }
  .knob.on {
    transform: translateX(20px);
    background: var(--primary);
  }
  .ghost {
    flex-shrink: 0;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
    color: var(--foreground);
    cursor: pointer;
  }
  .ghost.danger {
    color: var(--destructive);
    border-color: color-mix(in srgb, var(--destructive) 45%, transparent);
  }
  .ghost:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .err {
    margin: 0;
    color: var(--destructive);
  }
</style>
