<script lang="ts">
  // ExtensionManagerPage — popup "Ekstensi" (hash #extensions), 1:1 alur mobile:
  // daftar URL manifest → fetch daftar → install/uninstall per sumber + install zip.
  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type { ManifestEntry, ZipPreview } from "../domain/types";
  import { sourceStore } from "../stores/source.svelte";

  let manifestUrl = $state("");
  let entries = $state<ManifestEntry[]>([]);
  let zipUrl = $state("");
  let loading = $state(false);
  let busy = $state<string | null>(null);
  let err = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let zipPreview = $state<ZipPreview | null>(null);
  let zipSelected = $state<string[]>([]);

  const installedIds = $derived(
    new Set(sourceStore.available.map((s) => s.id)),
  );
  const installedSources = $derived(
    sourceStore.available.filter((source) => source.id !== "semua"),
  );

  onMount(() => {
    sourceStore.load();
  });

  async function fetchManifest() {
    if (!manifestUrl.trim()) {
      err = "Isi URL manifest terlebih dahulu.";
      entries = [];
      return;
    }
    loading = true;
    err = null;
    notice = null;
    try {
      const m = await api.extManifest(manifestUrl || undefined);
      entries = m.installableSources;
      if (entries.length === 0) notice = "Manifest kosong.";
    } catch (e) {
      err = `gagal ambil manifest: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function install(id: string) {
    if (!manifestUrl.trim()) {
      err = "Isi URL manifest terlebih dahulu.";
      return;
    }
    busy = id;
    err = null;
    notice = null;
    try {
      const s = await api.extInstall(manifestUrl, id);
      notice = `'${s.id}' v${s.version} terpasang.`;
      await sourceStore.load();
    } catch (e) {
      err = `install gagal: ${e}`;
    } finally {
      busy = null;
    }
  }

  async function uninstall(id: string) {
    busy = id;
    err = null;
    notice = null;
    try {
      await api.extUninstall(id);
      notice = `'${id}' dihapus.`;
      await sourceStore.load();
    } catch (e) {
      err = `uninstall gagal: ${e}`;
    } finally {
      busy = null;
    }
  }

  function prepareZip(preview: ZipPreview) {
    zipPreview = preview;
    zipSelected = [];
  }

  async function previewZip() {
    if (!zipUrl.trim()) return;
    busy = "zip";
    err = null;
    notice = null;
    try {
      prepareZip(await api.extPreviewZip(zipUrl.trim()));
      notice = "Pilih sumber yang ingin dipasang.";
    } catch (e) {
      err = `install zip gagal: ${e}`;
    } finally {
      busy = null;
    }
  }

  async function previewZipFile() {
    busy = "zipfile";
    err = null;
    notice = null;
    try {
      prepareZip(await api.extPreviewZipFile());
      notice = "Pilih sumber yang ingin dipasang.";
    } catch (e) {
      err = `install zip gagal: ${e}`;
    } finally {
      busy = null;
    }
  }

  function toggleZipSource(id: string) {
    zipSelected = zipSelected.includes(id)
      ? zipSelected.filter((selected) => selected !== id)
      : [...zipSelected, id];
  }

  async function installSelectedZip() {
    if (!zipPreview || zipSelected.length === 0) return;
    busy = "zip-install";
    err = null;
    notice = null;
    try {
      const ids = await api.extInstallStagedZip(zipPreview.token, zipSelected);
      notice = ids.length > 0 ? `terpasang: ${ids.join(", ")}` : "tak ada sumber dipasang.";
      zipPreview = null;
      zipSelected = [];
      await sourceStore.load();
    } catch (e) {
      err = `install zip gagal: ${e}`;
    } finally {
      busy = null;
    }
  }

  function metaLine(e: ManifestEntry): string {
    const m = e.meta;
    const lang = m?.language ? ` • ${m.language}` : "";
    return `${e.id} • v${e.version}${lang}`;
  }
</script>

<main class="ext">
  <header>
    <h1>Ekstensi</h1>
    <p>Pasang sumber dari repositori (ala mobile: Settings › Sources › Extension Repository). NHentai bawaan aplikasi.</p>
  </header>

  <label class="field">
    <span>URL manifest</span>
    <div class="rowline">
      <input bind:value={manifestUrl} placeholder="https://…/manifest.json" />
      <button onclick={fetchManifest} disabled={loading}>
        {loading ? "…" : "Muat"}
      </button>
    </div>
  </label>

  {#if err}
    <p class="err">{err}</p>
  {/if}
  {#if notice}
    <p class="ok">{notice}</p>
  {/if}

  <h2>Terpasang ({installedSources.length})</h2>
  <div class="list">
    {#each installedSources as source (source.id)}
      <div class="card">
        {#if source.iconUrl}
          <img class="source-icon" src={source.iconUrl} alt="" />
        {:else}
          <span class="source-icon icon-fallback">
            {source.label.slice(0, 1).toUpperCase()}
          </span>
        {/if}
        <div class="info">
          <strong>{source.label}</strong>
          <small>{source.id}{source.version ? ` • ${source.version}` : ""}</small>
        </div>
        <button
          class="ghost danger"
          onclick={() => uninstall(source.id)}
          disabled={busy === source.id || source.id === "nhentai"}
          title={source.id === "nhentai" ? "Bawaan aplikasi" : "Hapus sumber"}
        >
          {busy === source.id ? "…" : source.id === "nhentai" ? "Bawaan" : "Hapus"}
        </button>
      </div>
    {:else}
      <p class="muted">
        {sourceStore.loading ? "Memuat…" : "Belum ada sumber terpasang."}
      </p>
    {/each}
  </div>

  <h2>Tersedia ({entries.length})</h2>
  <div class="list">
    {#each entries as e (e.id)}
      {@const done = installedIds.has(e.id)}
      <div class="card">
        <div class="info">
          <strong>{e.meta?.displayName ?? e.id}</strong>
          <small>{metaLine(e)}</small>
          {#if e.meta?.description}
            <small class="desc">{e.meta.description}</small>
          {/if}
        </div>
        {#if done}
          <button
            class="ghost danger"
            onclick={() => uninstall(e.id)}
            disabled={busy === e.id || e.id === "nhentai"}
            title={e.id === "nhentai" ? "Bawaan aplikasi" : "Hapus"}
          >
            {busy === e.id ? "…" : e.id === "nhentai" ? "Bawaan" : "Hapus"}
          </button>
        {:else}
          <button class="ghost" onclick={() => install(e.id)} disabled={busy === e.id}>
            {busy === e.id ? "…" : "Pasang"}
          </button>
        {/if}
      </div>
    {:else}
      <p class="muted">{loading ? "Memuat…" : "Belum ada daftar. Isi URL manifest lalu Muat."}</p>
    {/each}
  </div>

  <h2>Install dari zip</h2>
  <div class="rowline">
    <button onclick={previewZipFile} disabled={busy === "zipfile"}>
      {busy === "zipfile" ? "…" : "Pilih file .zip…"}
    </button>
  </div>
  <div class="rowline" style="margin-top: 8px;">
    <input bind:value={zipUrl} placeholder="…atau tempel URL https://…/ekstensi.zip" />
    <button onclick={previewZip} disabled={busy === "zip" || !zipUrl.trim()}>
      {busy === "zip" ? "…" : "Muat zip"}
    </button>
  </div>
  {#if zipPreview}
    <div class="zip-preview">
      <strong>Pilih sumber yang akan dipasang</strong>
      {#each zipPreview.sources as source (source.id)}
        <label class="zip-source">
          <input
            type="checkbox"
            checked={zipSelected.includes(source.id)}
            onchange={() => toggleZipSource(source.id)}
          />
          {#if source.iconUrl}
            <img src={source.iconUrl} alt="" />
          {:else}
            <span class="icon-fallback">{source.id.slice(0, 1).toUpperCase()}</span>
          {/if}
          <span class="zip-source-info">
            <strong>{source.displayName ?? source.id}</strong>
            <small>{source.id} • v{source.version}</small>
          </span>
        </label>
      {:else}
        <p class="muted">Zip tak berisi config sumber.</p>
      {/each}
      <div class="zip-actions">
        <button class="ghost" onclick={() => (zipPreview = null)}>Batal</button>
        <button
          class="ghost"
          onclick={installSelectedZip}
          disabled={busy === "zip-install" || zipSelected.length === 0}
        >
          {busy === "zip-install" ? "…" : `Pasang terpilih (${zipSelected.length})`}
        </button>
      </div>
    </div>
  {/if}
</main>

<style>
  .ext {
    min-height: 100vh;
    padding: 20px;
    background: var(--background);
  }
  header h1 {
    margin: 0 0 4px;
    font-size: 20px;
  }
  header p {
    margin: 0 0 16px;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  h2 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted-foreground);
    margin: 20px 0 8px;
  }
  .field span {
    display: block;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 6px;
  }
  .rowline {
    display: flex;
    gap: 8px;
  }
  .rowline input {
    flex: 1;
    min-width: 0;
    padding: 8px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--input);
    background: var(--muted);
  }
  button {
    padding: 8px 16px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--card);
    cursor: pointer;
    font-weight: 600;
    white-space: nowrap;
  }
  button:disabled {
    opacity: 0.6;
    cursor: wait;
  }
  .ghost {
    border-color: var(--primary);
    color: var(--primary);
  }
  .ghost.danger {
    border-color: var(--destructive);
    color: var(--destructive);
  }
  .err {
    color: var(--destructive);
    font-size: 13px;
  }
  .ok {
    color: var(--primary);
    font-size: 13px;
  }
  .muted {
    color: var(--muted-foreground);
    font-size: 13px;
  }
  .zip-preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--card);
  }
  .zip-source {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: 8px;
    cursor: pointer;
  }
  .zip-source:hover {
    background: var(--muted);
  }
  .zip-source img,
  .icon-fallback {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    object-fit: cover;
    flex: 0 0 32px;
  }
  .icon-fallback {
    display: grid;
    place-items: center;
    background: var(--primary);
    color: var(--primary-foreground);
    font-weight: 700;
  }
  .zip-source-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .zip-source-info small {
    color: var(--muted-foreground);
  }
  .zip-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--card);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .info strong {
    font-size: 15px;
  }
  .info small {
    font-size: 12px;
    color: var(--muted-foreground);
  }
  .info .desc {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .source-icon {
    width: 36px;
    height: 36px;
    flex: 0 0 36px;
    border-radius: 9px;
    object-fit: cover;
  }
  .icon-fallback {
    display: grid;
    place-items: center;
    background: var(--primary);
    color: var(--primary-foreground);
    font-weight: 700;
  }
</style>
