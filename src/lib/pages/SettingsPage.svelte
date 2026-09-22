<script lang="ts">
  // SettingsPage — halaman Pengaturan (9.1 + 9.2, spec `settings-ai`):
  // seksi tampilan (tema + blur thumbnail default-on), pembaca, jaringan,
  // AI provider BYOK (keychain), data. Preferensi persist di localStorage
  // (`kuron.settings`).
  import { settingsStore } from "../stores/settings.svelte";
  import { themeStore, MODES } from "../stores/theme.svelte";
  import { platformStore } from "../stores/platform.svelte";
  import { libraryStore } from "../stores/library.svelte";
  import { aiProvidersStore } from "../stores/aiProviders.svelte";
  import { api } from "../api/client";
  import type { ReaderMode } from "../stores/settingsPersist";
  import type { AiModelOption, AiProviderKind } from "../domain/types";
  import { tagColor } from "../theme/tokens";

  const READER_MODES: { id: ReaderMode; label: string }[] = [
    { id: "paginated", label: "Per halaman" },
    { id: "continuous", label: "Gulir terus" },
    { id: "webtoon", label: "Webtoon" },
  ];

  const AI_KINDS: { id: AiProviderKind; label: string }[] = [
    { id: "openai", label: "OpenAI" },
    { id: "gemini", label: "Gemini" },
    { id: "cohere", label: "Cohere" },
    { id: "custom", label: "Kustom (OpenAI-compatible)" },
  ];

  const AI_PRESETS: Record<
    AiProviderKind,
    { base: string; model: string }
  > = {
    openai: { base: "https://api.openai.com/v1", model: "gpt-4o-mini" },
    gemini: {
      base: "https://generativelanguage.googleapis.com/v1beta",
      model: "gemini-2.0-flash",
    },
    cohere: { base: "https://api.cohere.com/v2", model: "command-r" },
    custom: { base: "", model: "" },
  };

  let runtimeLabel = $derived(
    platformStore.snapshot.runtime === "tauri" ? "Aplikasi desktop" : "Mode web",
  );
  let isDesktop = $derived(platformStore.snapshot.runtime === "tauri");
  let aiKind = $state<AiProviderKind>("openai");
  let aiName = $state("");
  let aiBaseUrl = $state(AI_PRESETS.openai.base);
  let aiModel = $state(AI_PRESETS.openai.model);
  let aiKey = $state("");
  let aiSaving = $state(false);
  // LOV model live (endpoint khusus per provider, ala mobile) — tanpa fallback.
  let aiModels = $state<AiModelOption[] | null>(null);
  let aiModelsLoading = $state(false);
  let aiModelsError = $state<string | null>(null);
  let aiManualModel = $state(false);

  $effect(() => {
    void aiProvidersStore.load();
  });

  function resetAiModels() {
    aiModels = null;
    aiModelsError = null;
    aiModelsLoading = false;
    aiManualModel = false;
  }

  function kindLabel(kind: AiProviderKind): string {
    return AI_KINDS.find((k) => k.id === kind)?.label ?? kind;
  }

  // Warna tile per jenis dari tag palette Kuron (theme-aware, bukan warna baru).
  const KIND_TAG: Record<AiProviderKind, string> = {
    openai: "uploader",
    gemini: "character",
    cohere: "language",
    custom: "other",
  };

  function kindColor(kind: AiProviderKind): string {
    return tagColor(KIND_TAG[kind], themeStore.darkMode);
  }

  function nameInitial(name: string): string {
    return name.trim().charAt(0).toUpperCase() || "?";
  }

  function onAiKindChange(k: AiProviderKind) {
    aiKind = k;
    aiBaseUrl = AI_PRESETS[k].base;
    aiModel = AI_PRESETS[k].model;
    resetAiModels();
  }

  function onAiKeyChange() {
    // Kunci berubah → daftar lama basi.
    resetAiModels();
  }

  async function loadAiModels() {
    if (aiModelsLoading) return;
    if (aiKind === "custom" || !aiKey.trim()) return;
    aiModelsLoading = true;
    aiModelsError = null;
    try {
      aiModels = await api.aiModelCatalog(aiKind, aiKey.trim());
      // Pastikan nilai tersimpan/terpilih ikut tampil walau tak di daftar.
      if (
        aiModel &&
        aiModels.length > 0 &&
        !aiModels.some((m) => m.id === aiModel)
      ) {
        aiModels = [
          { id: aiModel, label: null, is_vision: null },
          ...aiModels,
        ];
      }
    } catch (e) {
      aiModels = null;
      aiModelsError =
        e instanceof Error ? e.message : "gagal memuat daftar model";
    } finally {
      aiModelsLoading = false;
    }
  }

  async function saveAiProvider() {
    if (aiSaving) return;
    aiSaving = true;
    try {
      await aiProvidersStore.save(
        {
          id: null,
          name: aiName,
          kind: aiKind,
          base_url: aiBaseUrl,
          model: aiModel,
        },
        aiKey,
      );
      aiName = "";
      aiKey = "";
      resetAiModels();
      aiModel = AI_PRESETS[aiKind].model;
    } catch {
      // error → aiProvidersStore.error tampil di atas
    } finally {
      aiSaving = false;
    }
  }

  async function removeAiProvider(id: string) {
    if (!confirm("Hapus provider beserta kuncinya?")) return;
    try {
      await aiProvidersStore.remove(id);
    } catch {
      // error → store.error
    }
  }

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
  {#if aiProvidersStore.error}
    <p class="err">{aiProvidersStore.error}</p>
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

  <article class="card ai-card">
    <div class="ai-head">
      <h3>AI · Terjemahan</h3>
      {#if !isDesktop}
        <span class="pill warn">Mode web</span>
      {:else if aiProvidersStore.ready}
        <span class="pill ok">Aktif · {aiProvidersStore.readyCount} kunci</span>
      {:else}
        <span class="pill off">Nonaktif</span>
      {/if}
    </div>
    <div class="row">
      <span class="ai-glyph halftone" aria-hidden="true">AI</span>
      <div class="txt">
        {#if !isDesktop}
          <strong>Kelola kunci di aplikasi desktop</strong>
          <p class="hint">
            Keychain OS tidak tersedia di browser — terjemahan nonaktif di
            mode web.
          </p>
        {:else if aiProvidersStore.ready}
          <strong>Siap dipakai reader</strong>
          <p class="hint ok">
            {aiProvidersStore.readyCount} provider dengan kunci di keychain OS.
            Pemilihan provider di reader menyusul bersama terjemahan (tugas
            7.2).
          </p>
        {:else}
          <strong>Belum ada kunci API</strong>
          <p class="hint">
            Tanpa kunci provider, fitur terjemahan mati. Simpan kunci di bawah
            — hanya dititipkan ke keychain, bukan file.
          </p>
        {/if}
      </div>
    </div>
    {#each aiProvidersStore.providers as p (p.id)}
      <div class="row">
        <span
          class="tile"
          style:--tile={kindColor(p.kind)}
          aria-hidden="true">{nameInitial(p.name)}</span
        >
        <div class="txt">
          <strong>{p.name}</strong>
          <p class="hint meta">
            <span class="chip" style:--tile={kindColor(p.kind)}
              >{kindLabel(p.kind)}</span
            >
            <code>{p.model}</code>
            {#if p.has_key}
              <span class="pill tiny ok">Keychain</span>
            {:else}
              <span class="pill tiny warn">Tanpa kunci</span>
            {/if}
          </p>
        </div>
        <button
          type="button"
          class="ghost danger"
          onclick={() => removeAiProvider(p.id)}
        >
          Hapus
        </button>
      </div>
    {/each}
    {#if isDesktop}
      <div class="ai-form">
        <p class="form-kicker">Pasang kunci baru</p>
        <label class="field">
          <span>Jenis provider</span>
          <select
            value={aiKind}
            onchange={(e) =>
              onAiKindChange(e.currentTarget.value as AiProviderKind)}
          >
            {#each AI_KINDS as k (k.id)}
              <option value={k.id}>{k.label}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Nama tampil</span>
          <input
            type="text"
            placeholder="mis. Kunci Pribadi"
            bind:value={aiName}
          />
        </label>
        <label class="field wide">
          <span>Base URL</span>
          <input type="text" bind:value={aiBaseUrl} />
        </label>
        <label class="field wide">
          <span>Kunci API</span>
          <input
            type="password"
            placeholder="tidak pernah ditampilkan lagi"
            autocomplete="off"
            bind:value={aiKey}
            oninput={onAiKeyChange}
          />
        </label>
        {#if aiKey.trim()}
          <div class="model-block reveal">
            <div class="field">
              <span>Model</span>
              {#if aiKind === "custom"}
                <input
                  type="text"
                  placeholder="ID model, mis. llama-3.3-70b-instruct"
                  bind:value={aiModel}
                />
              {:else if aiManualModel}
                <input type="text" bind:value={aiModel} />
              {:else if aiModels}
                <select
                  value={aiModel}
                  onchange={(e) => (aiModel = e.currentTarget.value)}
                >
                  {#if !aiModel}
                    <option value="" disabled>pilih model…</option>
                  {/if}
                  {#each aiModels as m (m.id)}
                    <option value={m.id}>
                      {m.label ?? m.id}{m.is_vision === true
                        ? " · vision"
                        : m.is_vision === false
                          ? " · teks"
                          : ""}
                    </option>
                  {/each}
                </select>
              {:else}
                <button
                  type="button"
                  class="ghost load"
                  disabled={aiModelsLoading}
                  onclick={loadAiModels}
                >
                  {aiModelsLoading ? "Memuat…" : "Muat daftar model"}
                </button>
              {/if}
            </div>
            {#if aiKind === "custom"}
              <small class="tip"
                >Provider kustom tanpa endpoint daftar model — ketik ID model manual.</small
              >
            {:else if aiManualModel}
              <div class="model-tools">
                <button
                  type="button"
                  class="linkish"
                  onclick={() => {
                    aiManualModel = false;
                    if (!aiModels) void loadAiModels();
                  }}
                >
                  Pakai daftar model
                </button>
              </div>
            {:else if aiModels}
              <div class="model-tools">
                <span class="tip"
                  >{aiModels.length} model dari endpoint provider.</span
                >
                <button
                  type="button"
                  class="linkish"
                  disabled={aiModelsLoading}
                  onclick={loadAiModels}
                >
                  {aiModelsLoading ? "Memuat ulang…" : "Muat ulang"}
                </button>
                <button type="button" class="linkish" onclick={() => (aiManualModel = true)}>
                  Input manual
                </button>
              </div>
            {:else}
              <div class="model-tools">
                <button type="button" class="linkish" onclick={() => (aiManualModel = true)}>
                  Input manual
                </button>
              </div>
            {/if}
            {#if aiModelsError}
              <div class="model-tools">
                <span class="err-inline">{aiModelsError}</span>
                <button
                  type="button"
                  class="linkish"
                  disabled={aiModelsLoading}
                  onclick={loadAiModels}
                >
                  Coba lagi
                </button>
              </div>
            {/if}
          </div>
        {/if}
        <div class="ai-actions">
          <button
            type="button"
            class="save"
            disabled={aiSaving ||
              !aiName.trim() ||
              !aiKey.trim() ||
              (aiKind === "custom" && !aiModel.trim())}
            onclick={saveAiProvider}
          >
            {aiSaving ? "Menyimpan…" : "Simpan provider"}
          </button>
        </div>
      </div>
    {/if}
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
    flex: 1;
  }
  .txt strong {
    display: block;
    font-size: 14px;
    margin-bottom: 3px;
  }
  .ai-card .row .txt strong {
    font-size: 15px;
    font-weight: 800;
    letter-spacing: -0.01em;
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
  .hint.ok {
    color: var(--success);
  }

  /* —— AI · Terjemahan: identitas Kuron (tile, pill, halftone) —— */
  .ai-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .ai-glyph {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background-color: var(--primary);
    color: var(--primary-foreground);
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 20px;
    letter-spacing: 0.06em;
    line-height: 1;
  }
  .ai-glyph.halftone {
    /* halftone dots di atas coral, identik pola header Pengaturan */
    background-image: radial-gradient(
      color-mix(in srgb, var(--primary-foreground) 22%, transparent) 1.1px,
      transparent 1.3px
    );
    background-size: 6px 6px;
  }
  .tile {
    flex-shrink: 0;
    align-self: flex-start;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    margin-top: 1px;
    border-radius: 9px;
    border: 1px solid
      color-mix(in srgb, var(--tile, var(--border)) 55%, var(--border));
    background: color-mix(in srgb, var(--tile, var(--border)) 16%, var(--card));
    color: var(--tile, var(--foreground));
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 17px;
    letter-spacing: 0.04em;
    line-height: 1;
  }
  .hint.meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .chip {
    display: inline-block;
    font-size: 10.5px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 7px;
    border-radius: 6px;
    border: 1px solid
      color-mix(in srgb, var(--tile, var(--border)) 55%, var(--border));
    background: color-mix(in srgb, var(--tile, var(--border)) 14%, transparent);
    color: var(--tile, var(--foreground));
  }
  .pill {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
  }
  .pill.ok {
    color: var(--success);
    border-color: color-mix(in srgb, var(--success) 50%, var(--border));
    background: color-mix(in srgb, var(--success) 10%, var(--background));
  }
  .pill.warn {
    color: var(--warning);
    border-color: color-mix(in srgb, var(--warning) 50%, var(--border));
    background: color-mix(in srgb, var(--warning) 10%, var(--background));
  }
  .pill.off {
    color: var(--muted-foreground);
    border-style: dashed;
  }
  .pill.tiny {
    font-size: 9.5px;
    padding: 2px 7px;
    letter-spacing: 0.06em;
  }
  .form-kicker {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    font-family: "Komika", system-ui, sans-serif;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .form-kicker::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .save {
    background: var(--primary);
    color: var(--primary-foreground);
    border: 1px solid var(--primary);
    border-radius: 10px;
    padding: 9px 22px;
    font-size: 13.5px;
    font-weight: 800;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: filter 140ms ease;
  }
  .save:hover:not(:disabled) {
    filter: brightness(1.05);
  }
  /* Disabled: netral (muted), bukan coral pudar yang jadi cokelat muddy. */
  .save:disabled {
    background: var(--card);
    color: var(--muted-foreground);
    border-color: var(--border);
    cursor: default;
  }
  .save:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--primary) 55%, transparent);
    outline-offset: 2px;
  }

  .ai-form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px 16px;
    margin-top: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--muted);
  }
  .ai-form .field input,
  .ai-form .field select {
    background: var(--card);
  }
  .ai-form .ghost.load {
    background: var(--card);
  }
  .field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .field.wide {
    grid-column: 1 / -1;
  }
  .ai-actions {
    grid-column: 1 / -1;
    display: flex;
    justify-content: flex-end;
    padding-top: 2px;
  }
  @media (max-width: 560px) {
    .ai-form {
      grid-template-columns: 1fr;
    }
  }
  .field span {
    font-size: 12px;
    font-weight: 700;
    color: var(--muted-foreground);
  }
  .field input,
  .field select {
    height: 38px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--card);
    color: var(--foreground);
    font-size: 13px;
    padding: 0 12px;
    transition:
      border-color 140ms ease,
      box-shadow 140ms ease;
  }
  .field input:hover,
  .field select:hover {
    border-color: color-mix(in srgb, var(--foreground) 28%, transparent);
  }
  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 20%, transparent);
  }
  /* LOV (dropdown): chevron custom — appearance default beda tiap OS */
  .field select {
    appearance: none;
    -webkit-appearance: none;
    padding-right: 34px;
    cursor: pointer;
  }
  .field:has(> select)::after {
    content: "";
    position: absolute;
    right: 13px;
    bottom: 14px;
    width: 7px;
    height: 7px;
    border-right: 2px solid var(--muted-foreground);
    border-bottom: 2px solid var(--muted-foreground);
    transform: rotate(45deg);
    pointer-events: none;
  }
  .field select:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .reveal {
    animation: reveal 180ms ease;
  }
  @keyframes reveal {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .tip {
    font-size: 11.5px;
    color: var(--muted-foreground);
  }
  .model-block {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .model-tools {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .ghost.load {
    height: 38px;
    width: 100%;
    justify-content: center;
    border-style: dashed;
    color: var(--muted-foreground);
  }
  .linkish {
    border: 0;
    background: none;
    padding: 0;
    font-size: 12px;
    font-weight: 700;
    color: var(--primary);
    cursor: pointer;
  }
  .linkish:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .err-inline {
    font-size: 12px;
    color: var(--destructive);
  }
  .err {
    margin: 0;
    color: var(--destructive);
  }
</style>
