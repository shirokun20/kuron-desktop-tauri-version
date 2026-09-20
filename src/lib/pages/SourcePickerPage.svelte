<script lang="ts">
  // SourcePickerPage — daftar sumber "Pilih Sumber". Dipakai DUA cara dengan
  // hasil tap yang sama (`sourceStore.select`):
  //  - popup window native (#source-picker, desktop Tauri) → emit + tutup window
  //  - overlay in-app (web/mobile) → cukup panggil `onClose`
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { isTauriRuntime } from "../api/platform";
  import { sourceStore } from "../stores/source.svelte";
  import SourceList from "../components/SourceList.svelte";

  type Props = {
    onClose?: () => void | Promise<void>;
  };
  let { onClose }: Props = $props();

  let err = $state<string | null>(null);

  onMount(() => {
    sourceStore.load();
  });

  /** Mode web: halaman hash (#source-picker) ditutup dengan kembali ke root. */
  function leaveHashRoute() {
    if (window.location.hash) window.location.hash = "";
  }

  async function pick(id: string) {
    sourceStore.select(id);
    if (onClose) {
      await onClose();
      return;
    }
    if (!isTauriRuntime()) {
      leaveHashRoute();
      return;
    }
    try {
      await emit("source-selected", id);
    } catch (e) {
      err = `emit gagal: ${e}`;
      return;
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
  <SourceList sources={sourceStore.available} active={sourceStore.current} onSelect={pick} />
  {#if sourceStore.error}
    <p class="err">{sourceStore.error}</p>
  {/if}
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
