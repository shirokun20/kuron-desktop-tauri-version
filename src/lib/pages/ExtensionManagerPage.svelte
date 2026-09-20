<script lang="ts">
  // ExtensionManagerPage — popup "Ekstensi" (hash #extensions), 1:1 alur mobile:
  // daftar URL manifest → fetch daftar → install/uninstall per sumber + install zip.
  import { onMount } from "svelte";
  import { api, DEFAULT_MANIFEST_URL } from "../api/client";
  import type { ManifestEntry } from "../domain/types";
  import { sourceStore } from "../stores/source.svelte";

  let manifestUrl = $state(DEFAULT_MANIFEST_URL);
  let entries = $state<ManifestEntry[]>([]);
  let zipUrl = $state("");
  let loading = $state(false);
  let busy = $state<string | null>(null);
  let err = $state<string | null>(null);
  let notice = $state<string | null>(null);

  const installedIds = $derived(
    new Set(sourceStore.available.map((s) => s.id)),
  );

  onMount(() => {
    sourceStore.load();
    fetchManifest();
  });

  async function fetchManifest() {
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

  async function installZip() {
    if (!zipUrl.trim()) return;
    busy = "zip";
    err = null;
    notice = null;
    try {
      const ids = await api.extInstallZip(zipUrl.trim());
      notice = ids.length > 0 ? `terpasang: ${ids.join(", ")}` : "zip tak berisi config.";
      await sourceStore.load();
    } catch (e) {
      err = `install zip gagal: ${e}`;
    } finally {
      busy = null;
    }
  }

  async function installZipFile() {
    busy = "zipfile";
    err = null;
    notice = null;
    try {
      const ids = await api.extInstallZipFile();
      notice = ids.length > 0 ? `terpasang: ${ids.join(", ")}` : "zip tak berisi config.";
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
    <button onclick={installZipFile} disabled={busy === "zipfile"}>
      {busy === "zipfile" ? "…" : "Pilih file .zip…"}
    </button>
  </div>
  <div class="rowline" style="margin-top: 8px;">
    <input bind:value={zipUrl} placeholder="…atau tempel URL https://…/ekstensi.zip" />
    <button onclick={installZip} disabled={busy === "zip" || !zipUrl.trim()}>
      {busy === "zip" ? "…" : "Pasang"}
    </button>
  </div>
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
</style>
