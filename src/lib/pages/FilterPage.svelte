<script lang="ts">
  // FilterPage — Layar Search mobile (form per-sumber, teks PALING ATAS).
  // Port DynamicFormSearchUI: label manusiawi, _formatFieldValue
  // (transform/quote/prefix/suffix), split koma, joinMode, preview query.
  // Submit → `raw:..` via event 'filter-submit' → tutup diri.
  import { onMount } from "svelte";
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "../api/client";
  import { sourceStore } from "../stores/source.svelte";

  type Option = { value: string; label: string; group?: string };

  let form = $state<Record<string, any> | null>(null);
  let error = $state<string | null>(null);
  let values = $state<Record<string, string | string[]>>({});
  let openGroups = $state<Record<string, boolean>>({});

  const sourceLabel = sourceStore.currentLabel;

  const ID_LABELS: Record<string, string> = {
    query: "Kata kunci",
    title: "Judul",
    sort: "Urutkan",
    status: "Status",
    genre: "Genre",
    tag: "Tag",
    artist: "Artis",
    author: "Penulis",
    character: "Karakter",
    parody: "Parodi",
    group: "Grup",
    language: "Bahasa",
    category: "Kategori",
    order: "Urutan",
    contentRating: "Rating konten",
    publicationDemographic: "Demografi",
    includedTag: "Tag disertakan",
    excludedTag: "Tag dikecualikan",
    includedTagsMode: "Mode tag+",
    excludedTagsMode: "Mode tag−",
    hasAvailableChapters: "Ada chapter",
    originalLanguage: "Bahasa asli",
    availableTranslatedLanguage: "Terjemahan tersedia",
    year: "Tahun",
    createdAtSince: "Dibuat sejak",
    updatedAtSince: "Diperbarui sejak",
  };

  function humanize(key: string): string {
    if (ID_LABELS[key]) return ID_LABELS[key];
    return key
      .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
      .replace(/[_-]+/g, " ")
      .replace(/\w\S*/g, (w) => w.charAt(0).toUpperCase() + w.slice(1));
  }

  function labelFor(key: string, def: any): string {
    const raw = typeof def?.label === "string" ? def.label.trim() : "";
    return raw || humanize(key);
  }

  onMount(async () => {
    try {
      form = (await api.searchForm(sourceStore.current)) as Record<string, any> | null;
      for (const [key] of orderedEntries()) openGroups[key] = true;
    } catch (e) {
      error = `gagal muat form: ${e}`;
    }
  });

  function normOptions(list: any): Option[] {
    if (!Array.isArray(list)) return [];
    return list.flatMap((o: any) => {
      if (typeof o === "string") return [{ value: o, label: o }];
      if (o && typeof o === "object") {
        const v = o.value ?? o.slug ?? o.id;
        const l = o.label ?? o.name ?? v;
        if (v !== undefined) return [{ value: String(v), label: String(l), group: o.group }];
      }
      return [];
    });
  }

  function optionsFor(def: any): Option[] {
    const dsName = def?.ui?.dataSource;
    const ds = dsName ? form?.dataSources?.[dsName] : undefined;
    return normOptions(ds?.options ?? def?.options);
  }

  /** Teks dulu (kotak Search mobile), sisanya urutan config. */
  function orderedEntries(): [string, any][] {
    const params = form?.params ?? {};
    const all = Object.entries(params).filter(([, d]: [string, any]) => d?.type !== "page");
    const isText = ([, d]: [string, any]) => d?.type === "text";
    return [...all.filter(isText), ...all.filter((e) => !isText(e))];
  }

  function isMulti(def: any): boolean {
    return def?.type === "tag" || def?.type === "checkbox";
  }

  function selectedCount(key: string): number {
    const v = values[key];
    if (Array.isArray(v)) return v.filter(Boolean).length;
    return typeof v === "string" && v.trim() ? 1 : 0;
  }

  function toggle(key: string, value: string) {
    const cur = values[key];
    const arr = Array.isArray(cur) ? [...cur] : [];
    values[key] = arr.includes(value) ? arr.filter((v) => v !== value) : [...arr, value];
  }

  /** Port `_formatFieldValue`: transform/quote/prefix/suffix. */
  function formatValue(def: any, value: string): string {
    let r = value.trim();
    if (!r) return r;
    const t = (def?.transform ?? "").trim();
    if (t === "lowercase") r = r.toLowerCase();
    else if (t === "uppercase") r = r.toUpperCase();
    else if (t === "spaceToPlus") r = r.replaceAll(" ", "+");
    if (def?.quoteIfContainsSpace === true && r.includes(" ") && !r.startsWith('"')) {
      r = `"${r}"`;
    }
    const pre = (def?.valuePrefix ?? "").trim();
    const suf = (def?.valueSuffix ?? "").trim();
    return `${pre}${r}${suf}`;
  }

  function splitInput(def: any, raw: string): string[] {
    const multi = def?.type === "tag" || def?.multiInput === true;
    if (!multi) return raw.trim() ? [raw.trim()] : [];
    return raw
      .split(/[\n,]+/)
      .map((s) => s.trim())
      .filter(Boolean);
  }

  /** Bangun pasangan query (mobile `_collectEncodedQueryParts`). */
  function buildParts(): string[] {
    const byParam: Record<string, string[]> = {};
    const joinMode: Record<string, string> = {};
    for (const [key, def] of orderedEntries()) {
      const qp = def?.queryParam;
      if (!qp || qp === "rawParam") continue;
      if (def?.joinMode) joinMode[qp] = def.joinMode;
      const v = values[key];
      const push = (s: string) => {
        const f = formatValue(def, s);
        if (f) (byParam[qp] ??= []).push(f);
      };
      if (Array.isArray(v)) v.forEach(push);
      else if (typeof v === "string") splitInput(def, v).forEach(push);
    }
    // rawParam: nilai sudah pasangan jadi.
    for (const [key, def] of orderedEntries()) {
      if (def?.queryParam !== "rawParam") continue;
      const v = values[key];
      if (typeof v === "string" && v.trim()) {
        (byParam[`\0raw${key}`] ??= []).push(`\0${v.trim()}`);
      }
    }
    const parts: string[] = [];
    for (const [qp, vals] of Object.entries(byParam)) {
      if (vals.length === 0) continue;
      if (qp.startsWith("\0raw")) {
        parts.push(vals[0].slice(1));
        continue;
      }
      if ((joinMode[qp] ?? "").toLowerCase() === "space") {
        parts.push(`${encodeURIComponent(qp)}=${encodeURIComponent(vals.join(" "))}`);
        continue;
      }
      for (const x of vals) {
        parts.push(`${encodeURIComponent(qp)}=${encodeURIComponent(x)}`);
      }
    }
    return parts;
  }

  function preview(): string {
    const p = buildParts();
    return p.length ? `raw:${p.join("&")}` : "";
  }

  function reset() {
    values = {};
  }

  async function close() {
    try {
      await (await getCurrentWindow()).close();
    } catch {
      // mode browser: abaikan
    }
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    const parts = buildParts();
    if (!parts.length) return;
    const texts = orderedEntries()
      .filter(([, d]) => d?.type === "text")
      .map(([k]) => (typeof values[k] === "string" ? (values[k] as string).trim() : ""))
      .filter(Boolean);
    try {
      await emit("filter-submit", {
        query: `raw:${parts.join("&")}`,
        label: texts[0] || `${parts.length} kriteria`,
      });
    } catch (e) {
      error = `emit gagal: ${e}`;
      return;
    }
    await close();
  }
</script>

<main class="modal">
  <header>
    <div>
      <h1>Cari</h1>
      <p class="sub">{sourceLabel}</p>
    </div>
    <button class="icon-btn" onclick={close} title="Tutup">✕</button>
  </header>

  {#if error}
    <p class="err">{error}</p>
  {:else if !form}
    <p class="muted">Memuat form…</p>
  {:else if orderedEntries().length === 0}
    <p class="muted">Sumber ini tak punya form filter.</p>
  {:else}
    <form class="body" onsubmit={submit}>
      {#each orderedEntries() as [key, def]}
        {@const opts = optionsFor(def)}
        {@const n = selectedCount(key)}
        {#if def?.type === "select" || def?.type === "sort"}
          <label class="field">
            <span class="label">{labelFor(key, def)}</span>
            <select bind:value={values[key]}>
              <option value="">— Semua —</option>
              {#each opts as o}
                <option value={o.value}>{o.label}</option>
              {/each}
            </select>
          </label>
        {:else if isMulti(def) && opts.length > 0}
          <details class="group" open={openGroups[key] ?? true}>
            <summary>
              {labelFor(key, def)}
              {#if n > 0}<span class="count">{n}</span>{/if}
            </summary>
            <div class="pills">
              {#each opts as o}
                {@const on = Array.isArray(values[key]) && (values[key] as string[]).includes(o.value)}
                <button
                  type="button"
                  class={["pill", on ? "on" : ""].join(" ")}
                  aria-pressed={on}
                  onclick={() => toggle(key, o.value)}
                >
                  {o.label}
                </button>
              {/each}
            </div>
          </details>
        {:else}
          <label class="field">
            <span class="label">{labelFor(key, def)}</span>
            <input
              type="text"
              placeholder={def?.placeholder ?? labelFor(key, def)}
              bind:value={values[key]}
            />
            {#if def?.type === "tag"}
              <span class="hint">Pisahkan beberapa nilai dengan koma.</span>
            {/if}
          </label>
        {/if}
      {/each}
      {#if preview()}
        <p class="preview">{preview()}</p>
      {/if}
      <footer>
        <button type="button" class="ghost" onclick={reset}>Atur ulang</button>
        <span class="spacer"></span>
        <button type="button" class="ghost" onclick={close}>Batal</button>
        <button type="submit" class="primary">Cari</button>
      </footer>
    </form>
  {/if}
</main>

<style>
  .modal {
    padding: 24px 24px 0;
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding-bottom: 16px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    letter-spacing: -0.02em;
  }
  .sub {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  .icon-btn {
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--muted-foreground);
    cursor: pointer;
    font-size: 15px;
    padding: 6px 10px;
  }
  .icon-btn:hover {
    background: var(--muted);
    color: var(--foreground);
  }
  .body {
    overflow-y: auto;
    padding-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 13px;
    font-weight: 600;
  }
  .hint {
    font-size: 12px;
    color: var(--muted-foreground);
  }
  .field input,
  .field select {
    height: 40px;
    padding: 0 12px;
    border-radius: var(--radius);
    border: 1px solid var(--input);
    background: var(--muted);
    color: inherit;
    font-size: 14px;
  }
  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 22%, transparent);
  }
  .group {
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }
  .group summary {
    cursor: pointer;
    padding: 12px 14px;
    font-size: 13px;
    font-weight: 700;
    display: flex;
    justify-content: space-between;
    align-items: center;
    list-style: none;
  }
  .group summary::-webkit-details-marker {
    display: none;
  }
  .group summary:hover {
    background: var(--muted);
  }
  .count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--primary);
    color: var(--primary-foreground);
    font-size: 12px;
  }
  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 0 14px 14px;
    max-height: 220px;
    overflow-y: auto;
  }
  .pill {
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    color: inherit;
    font-size: 13px;
    padding: 5px 12px;
    cursor: pointer;
    transition: 150ms ease-out;
  }
  .pill:hover {
    border-color: var(--primary);
  }
  .pill.on {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
    font-weight: 600;
  }
  .preview {
    font-size: 12px;
    color: var(--muted-foreground);
    word-break: break-all;
    border-top: 1px solid var(--border);
    padding-top: 10px;
    margin: 0;
  }
  footer {
    display: flex;
    gap: 12px;
    padding: 12px 0 20px;
    position: sticky;
    bottom: 0;
    background: var(--background);
  }
  .spacer {
    flex: 1;
  }
  .primary {
    height: 40px;
    padding: 0 24px;
    border-radius: 10px;
    border: 1px solid transparent;
    background: var(--primary);
    color: var(--primary-foreground);
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
  }
  .primary:active {
    transform: scale(0.98);
  }
  .ghost {
    height: 40px;
    padding: 0 20px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: transparent;
    color: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
  }
  .err {
    color: var(--destructive);
  }
  .muted {
    color: var(--muted-foreground);
    font-size: 14px;
  }
</style>
