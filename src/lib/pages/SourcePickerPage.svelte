<script lang="ts">
  // SourcePickerPage — window popup native "Pilih Sumber" (hash #source-picker).
  // Pilih → emit 'source-selected' ke window main → tutup diri.
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { SOURCE_META, sourceStore } from "../stores/source.svelte";
  import SourceList from "../components/SourceList.svelte";

  let err = $state<string | null>(null);

  async function pick(label: string) {
    sourceStore.select(label);
    try {
      await emit("source-selected", label);
    } catch (e) {
      err = `emit gagal: ${e}`;
    }
    try {
      await getCurrentWindow().close();
    } catch (e) {
      err = `close gagal (butuh core:window:allow-close + restart dev): ${e}`;
    }
  }
</script>

<main class="picker">
  <header>
    <h1>Sumber</h1>
    <p>Ganti provider untuk feed, detail, pencarian, dan data reader.</p>
  </header>
  <SourceList sources={SOURCE_META} active={sourceStore.current} onSelect={pick} />
  {#if err}
    <p class="err">{err}</p>
  {/if}
</main>

<style>
  .picker {
    min-height: 100vh;
    padding: 20px;
    background: var(--background);
  }
  .err {
    color: var(--destructive);
    font-size: 13px;
    margin-top: 12px;
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
</style>
