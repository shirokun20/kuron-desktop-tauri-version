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

  type Props = {
    onClose?: () => void | Promise<void>;
  };
  let { onClose }: Props = $props();

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

  let reloading = $state(false);

  async function loadForm() {
    error = null;
    try {
      const raw = (await api.searchForm(sourceStore.current)) as any;
      // Backend: `{params, dataSources}` langsung ATAU `{searchForm: {...}}`.
      form = (raw?.params ? raw : raw?.searchForm ?? raw) as Record<string, any> | null;
      for (const [key] of visibleEntries()) openGroups[key] = true;
    } catch (e) {
      error = `gagal muat form: ${e}`;
    }
  }

  async function reload() {
    reloading = true;
    try {
      await loadForm();
    } finally {
      reloading = false;
    }
  }

  onMount(loadForm);

  /** Error fetch `dataSource` milik field ini (bila ada). */
  function dsError(def: any): string | null {
    const dsName = def?.ui?.dataSource;
    const err = dsName ? form?.dataSources?.[dsName]?.error : undefined;
    return typeof err === "string" && err ? err : null;
  }

  /** Ringkasan diagnostik: jumlah field + status tiap dataSource. */
  function dsSummary(): string {
    const ds = form?.dataSources ?? {};
    return Object.entries(ds)
      .map(([k, v]: [string, any]) =>
        Array.isArray(v?.options)
          ? `${k}:${v.options.length}${v?.error ? "(ERR)" : ""}`
          : `${k}:?`,
      )
      .join(" ");
  }

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
    if (def?.ui?.multi === true) return true;
    if (def?.type === "checkbox") return true;
    // `tag` + dataSource (picker multi ala mobile) ATAU tag tanpa options
    // inline (opsi datang dari dataSources ter-resolve).
    if (def?.type === "tag") {
      if (def?.ui?.dataSource) return true;
      if (!Array.isArray(def?.options)) return true;
    }
    return false;
  }

  /** Pilihan picker/checkbox: options inline atau `dataSources` ter-resolve. */
  function pickerOptions(def: any): Option[] {
    const ds = optionsFor(def);
    if (ds.length) return ds;
    return normOptions(def?.options);
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

  function arrOf(key: string): string[] {
    const v = values[key];
    return Array.isArray(v) ? v : [];
  }

  /* === Picker gabungan ala mobile (dynamic_form_search_ui.dart, DFU) === */
  // Syarat field picker DFU:1835-1844.
  function isPickerField(def: any): boolean {
    return def?.type === "tag" && def?.ui?.selector === "picker" && def?.ui?.multi === true;
  }
  // Deteksi include/exclude via substring nama ATAU queryParam DFU:1856-1868.
  function isIncludedField(key: string, def: any): boolean {
    const q = String(def?.queryParam ?? "").toLowerCase().replace(/[\s_[\]]+/g, "");
    return key.toLowerCase().includes("included") || q.includes("includedtags");
  }
  function isExcludedField(key: string, def: any): boolean {
    const q = String(def?.queryParam ?? "").toLowerCase().replace(/[\s_[\]]+/g, "");
    return key.toLowerCase().includes("excluded") || q.includes("excludedtags");
  }
  // Pasangan excluded pertama utk field included DFU:1886-1893.
  function pairedExcludedKey(incKey: string): string | null {
    for (const [k, d] of orderedEntries()) {
      if (k !== incKey && isPickerField(d) && isExcludedField(k, d)) return k;
    }
    return null;
  }
  // Field excluded disembunyikan dari daftar bila pasangannya ada DFU:1870-1875.
  // (Query tetap dikumpul dari orderedEntries, bukan dari sini.)
  function visibleEntries(): [string, any][] {
    const all = orderedEntries();
    const hasInc = all.some(([k, d]) => isPickerField(d) && isIncludedField(k, d));
    if (!hasInc) return all;
    return all.filter(([k, d]) => !(isPickerField(d) && isExcludedField(k, d)));
  }
  // Opsi modal: sumber field included, fallback excluded DFU:1223-1224.
  function pairOptions(incKey: string, excKey: string | null): Option[] {
    const o = pickerOptions(form?.params?.[incKey]);
    if (o.length || !excKey) return o;
    return pickerOptions(form?.params?.[excKey]);
  }
  function optLabel(opts: Option[], value: string): string {
    return opts.find((o) => o.value === value)?.label ?? value;
  }
  // Urutan grup mobile DFU:1297-1322 (sisa alfabetis).
  const GROUP_ORDER = ["format", "genre", "theme", "content", "other"];
  function groupedOptions(opts: Option[]): { group: string; items: Option[] }[] {
    const map = new Map<string, Option[]>();
    for (const o of opts) {
      const g = (o.group ?? "").trim().toLowerCase() || "other";
      if (!map.has(g)) map.set(g, []);
      map.get(g)!.push(o);
    }
    const rank = (g: string) => {
      const i = GROUP_ORDER.indexOf(g);
      return i === -1 ? GROUP_ORDER.length : i;
    };
    return [...map.entries()]
      .sort((a, b) => rank(a[0]) - rank(b[0]) || a[0].localeCompare(b[0]))
      .map(([group, items]) => ({ group, items }));
  }
  function capGroup(g: string): string {
    return g ? g.charAt(0).toUpperCase() + g.slice(1) : g;
  }

  type Pick = "include" | "exclude";
  let pickerOpen = $state<{ incKey: string; excKey: string | null; dual: boolean } | null>(null);
  let picks = $state<Record<string, Pick>>({});
  let pickerQuery = $state("");

  function openPicker(incKey: string, excKey: string | null) {
    const seed: Record<string, Pick> = {};
    for (const v of arrOf(incKey)) seed[v] = "include";
    if (excKey) for (const v of arrOf(excKey)) seed[v] = "exclude";
    picks = seed;
    pickerQuery = "";
    pickerOpen = { incKey, excKey, dual: excKey !== null };
  }
  function closePicker() {
    pickerOpen = null;
    pickerQuery = "";
  }
  // Siklus DFU:1480-1496: none → include → exclude → none (gabungan);
  // tunggal: toggle pilih.
  function cyclePick(value: string) {
    const dual = pickerOpen?.dual ?? false;
    const cur = picks[value];
    const next = { ...picks };
    if (!dual) {
      if (cur) delete next[value];
      else next[value] = "include";
    } else if (!cur) {
      next[value] = "include";
    } else if (cur === "include") {
      next[value] = "exclude";
    } else {
      delete next[value];
    }
    picks = next;
  }
  function pickCounts(): { inc: number; exc: number } {
    let inc = 0;
    let exc = 0;
    for (const p of Object.values(picks)) {
      if (p === "include") inc++;
      else exc++;
    }
    return { inc, exc };
  }
  // Opsi modal kini (cari substring label DFU:1387-1399).
  function modalOptions(): Option[] {
    if (!pickerOpen) return [];
    const all = pairOptions(pickerOpen.incKey, pickerOpen.excKey);
    const q = pickerQuery.trim().toLowerCase();
    if (!q) return all;
    return all.filter((o) => o.label.toLowerCase().includes(q));
  }
  function modalError(): string | null {
    if (!pickerOpen) return null;
    return (
      dsError(form?.params?.[pickerOpen.incKey]) ??
      (pickerOpen.excKey ? dsError(form?.params?.[pickerOpen.excKey]) : null)
    );
  }
  // Hint cari modal: "tag" utk gabungan, label field utk tunggal.
  function pickerSearchHint(): string {
    if (!pickerOpen) return "Cari…";
    if (pickerOpen.dual) return "Cari tag…";
    const label = labelFor(pickerOpen.incKey, form?.params?.[pickerOpen.incKey]);
    return `Cari ${label.toLowerCase()}…`;
  }
  // Terapkan + auto-AND mode bila seleksi tak kosong dan mode belum diisi
  // DFU:1548-1564 (desktop: hanya bila "AND" ada di opsi config).
  function applyPicker() {
    const ctx = pickerOpen;
    if (!ctx) return;
    const inc: string[] = [];
    const exc: string[] = [];
    for (const [v, p] of Object.entries(picks)) (p === "include" ? inc : exc).push(v);
    values[ctx.incKey] = inc;
    if (ctx.excKey) {
      values[ctx.excKey] = exc;
      autoAnd("includedTagsMode", inc.length > 0);
      autoAnd("excludedTagsMode", exc.length > 0);
    }
    closePicker();
  }
  function autoAnd(queryParam: string, need: boolean) {
    if (!need) return;
    for (const [k, d] of orderedEntries()) {
      if (d?.queryParam !== queryParam) continue;
      const cur = values[k];
      const empty = Array.isArray(cur) ? cur.length === 0 : !String(cur ?? "").trim();
      if (!empty) return;
      if (pickerOptions(d).some((o) => o.value === "AND")) values[k] = "AND";
      return;
    }
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

  async function closeWindow() {
    try {
      await (await getCurrentWindow()).close();
    } catch {
      // mode browser: abaikan
    }
  }

  async function close() {
    if (onClose) {
      await onClose();
      return;
    }
    await closeWindow();
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

<main class="modal" class:sheet-inline={!!onClose}>
  <header>
    <div>
      <h1>Cari</h1>
      <p class="sub">{sourceLabel}</p>
    </div>
    <button class="icon-btn" onclick={close} title="Tutup">✕</button>
  </header>
  {#if form}
    <p class="dbg">
      {Object.keys(form?.params ?? {}).length} field · {dsSummary() || "tanpa dataSource"}
      <button type="button" class="link" onclick={reload} disabled={reloading}>
        {reloading ? "Memuat…" : "Muat ulang"}
      </button>
    </p>
  {/if}

  {#if error}
    <p class="err">{error}</p>
  {:else if !form}
    <p class="muted">Memuat form…</p>
  {:else if visibleEntries().length === 0}
    <p class="muted">Sumber ini tak punya form filter.</p>
  {:else}
    <form class="body" onsubmit={submit}>
      {#each visibleEntries() as [key, def]}
        {@const opts = pickerOptions(def)}
        {@const n = selectedCount(key)}
        {@const excKey =
          isPickerField(def) && isIncludedField(key, def) ? pairedExcludedKey(key) : null}
        {@const pairOpts = excKey ? pairOptions(key, excKey) : opts}
        {@const pairErr =
          dsError(def) ?? (excKey ? dsError(form?.params?.[excKey]) : null)}
        {#if excKey}
          {@const ne = selectedCount(excKey)}
          <div class="picker-row">
            <button type="button" class="picker-open" onclick={() => openPicker(key, excKey)}>
              <span class="picker-label">{labelFor(key, def)}</span>
              <span class="picker-hint">
                <b class="inc">+</b> Sertakan {n} · <b class="exc">−</b> Kecualikan {ne}
              </span>
              <span class="chev">›</span>
            </button>
            {#if pairErr}
              <p class="err ds-err">
                {pairErr}
                <button type="button" class="link" onclick={reload} disabled={reloading}>
                  {reloading ? "Memuat…" : "Muat ulang"}
                </button>
              </p>
            {/if}
            {#if n + ne > 0}
              <div class="picked">
                {#each arrOf(key) as v}
                  <button
                    type="button"
                    class="chip inc"
                    title="Hapus"
                    onclick={() => toggle(key, v)}
                  >
                    + {optLabel(pairOpts, v)} ✕
                  </button>
                {/each}
                {#each arrOf(excKey) as v}
                  <button
                    type="button"
                    class="chip exc"
                    title="Hapus"
                    onclick={() => toggle(excKey, v)}
                  >
                    − {optLabel(pairOpts, v)} ✕
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {:else if isPickerField(def)}
          <div class="picker-row">
            <button type="button" class="picker-open" onclick={() => openPicker(key, null)}>
              <span class="picker-label">{labelFor(key, def)}</span>
              <span class="picker-hint">
                {#if n > 0}
                  {n} dipilih
                {:else}
                  {typeof def?.placeholder === "string" && def.placeholder
                    ? def.placeholder
                    : "Pilih"}
                {/if}
              </span>
              <span class="chev">›</span>
            </button>
            {#if pairErr}
              <p class="err ds-err">
                {pairErr}
                <button type="button" class="link" onclick={reload} disabled={reloading}>
                  {reloading ? "Memuat…" : "Muat ulang"}
                </button>
              </p>
            {/if}
            {#if n > 0}
              <div class="picked">
                {#each arrOf(key) as v}
                  <button
                    type="button"
                    class="chip"
                    title="Hapus"
                    onclick={() => toggle(key, v)}
                  >
                    {optLabel(opts, v)} ✕
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {:else if isMulti(def)}
          <details class="group" open={openGroups[key] ?? true}>
            <summary>
              {labelFor(key, def)}
              {#if n > 0}<span class="count">{n}</span>{/if}
            </summary>
            {#if dsError(def)}
              <p class="err ds-err">
                {dsError(def)}
                <button type="button" class="link" onclick={reload} disabled={reloading}>
                  {reloading ? "Memuat…" : "Muat ulang"}
                </button>
              </p>
            {/if}
            <div class="pills">
              {#if opts.length === 0 && !dsError(def)}
                <p class="muted">Opsi tak termuat — ketik manual di bawah lalu Enter.</p>
              {/if}
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
              {#if Array.isArray(values[key])}
                {#each (values[key] as string[]).filter((v) => !opts.some((o) => o.value === v)) as manual}
                  <button
                    type="button"
                    class="pill on"
                    title="Hapus"
                    onclick={() => toggle(key, manual)}
                  >
                    {manual} ✕
                  </button>
                {/each}
              {/if}
            </div>
            <label class="field manual">
              <span class="hint">Tambah manual (ID/slug, Enter)</span>
              <input
                type="text"
                placeholder={def?.placeholder ?? labelFor(key, def)}
                onkeydown={(e) => {
                  if (e.key !== "Enter") return;
                  e.preventDefault();
                  const t = (e.currentTarget as HTMLInputElement).value;
                  const cur = Array.isArray(values[key]) ? [...(values[key] as string[])] : [];
                  for (const s of splitInput(def, t)) if (!cur.includes(s)) cur.push(s);
                  values[key] = cur;
                  (e.currentTarget as HTMLInputElement).value = "";
                }}
              />
            </label>
          </details>
        {:else if def?.type === "select" || def?.type === "sort"}
          <label class="field">
            <span class="label">{labelFor(key, def)}</span>
            <select bind:value={values[key]}>
              <option value="">— Semua —</option>
              {#each opts as o}
                <option value={o.value}>{o.label}</option>
              {/each}
            </select>
          </label>
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

  {#if pickerOpen}
    {@const mc = pickCounts()}
    {@const mopts = modalOptions()}
    {@const merr = modalError()}
    {@const mgroups = groupedOptions(mopts)}
    {@const hideGroupHead = mgroups.length === 1 && mgroups[0].group === "other"}
    <div class="sheet-wrap">
      <button
        type="button"
        class="sheet-backdrop"
        onclick={closePicker}
        aria-label="Tutup"
      ></button>
      <div class="sheet" role="dialog" aria-modal="true" aria-label="Filter Tag">
        <header class="sheet-head">
          <h2>
            {pickerOpen.dual
              ? "Filter Tag"
              : labelFor(pickerOpen.incKey, form?.params?.[pickerOpen.incKey])}
          </h2>
          <span class="sheet-actions">
            <button
              type="button"
              class="link"
              onclick={() => (picks = {})}
              disabled={mc.inc + mc.exc === 0}
            >
              Atur ulang
            </button>
            <button type="button" class="icon-btn" onclick={closePicker} title="Tutup">✕</button>
          </span>
        </header>
        <p class="counts">
          {#if pickerOpen.dual}
            <b class="inc">+</b> Sertakan {mc.inc} · <b class="exc">−</b> Kecualikan {mc.exc}
          {:else}
            {mc.inc} dipilih
          {/if}
        </p>
        <input
          type="text"
          class="search"
          placeholder={pickerSearchHint()}
          bind:value={pickerQuery}
        />
        {#if merr}
          <p class="err ds-err">
            {merr}
            <button type="button" class="link" onclick={reload} disabled={reloading}>
              {reloading ? "Memuat…" : "Muat ulang"}
            </button>
          </p>
        {/if}
        <div class="sheet-body">
          {#if mopts.length === 0}
            <p class="muted">
              {pickerQuery.trim()
                ? "Tidak ditemukan"
                : merr
                  ? "Opsi gagal dimuat."
                  : "Tidak ada opsi."}
            </p>
          {/if}
          {#each mgroups as g}
            {#if !hideGroupHead}
              <h3>{capGroup(g.group)}</h3>
            {/if}
            <div class="picks">
              {#each g.items as o}
                {@const st = picks[o.value]}
                <button
                  type="button"
                  class={["pick", st === "include" ? "inc" : st === "exclude" ? "exc" : ""].join(
                    " ",
                  )}
                  aria-pressed={!!st}
                  onclick={() => cyclePick(o.value)}
                >
                  {#if st === "include"}
                    <span class="avatar">+</span>
                  {:else if st === "exclude"}
                    <span class="avatar">−</span>
                  {/if}
                  {o.label}
                </button>
              {/each}
            </div>
          {/each}
        </div>
        <footer class="sheet-foot">
          <button type="button" class="ghost" onclick={closePicker}>Batal</button>
          <button type="button" class="primary" onclick={applyPicker}>
            {#if pickerOpen.dual}
              Terapkan ({mc.inc} / {mc.exc})
            {:else}
              Terapkan ({mc.inc})
            {/if}
          </button>
        </footer>
      </div>
    </div>
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
  .modal.sheet-inline {
    height: min(92vh, 820px);
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
  /* Scroll container: kelebihan di-scroll, bukan disusutkan. Tanpa ini
   * anak ber-overflow:hidden (min-height:0) melumat sampai tinggi 0. */
  .body > * {
    flex-shrink: 0;
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
  .manual {
    padding: 0 14px 14px;
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
  .dbg {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--muted-foreground);
  }
  .ds-err {
    margin: 0 14px 8px;
    font-size: 12px;
  }
  .link {
    background: none;
    border: none;
    padding: 0 0 0 8px;
    color: var(--primary);
    font-size: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .link:disabled {
    opacity: 0.6;
    cursor: wait;
  }
  .muted {
    color: var(--muted-foreground);
    font-size: 14px;
  }

  /* === Baris picker + modal sheet ala mobile === */
  .picker-row {
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }
  .picker-open {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
  }
  .picker-open:hover {
    background: var(--muted);
  }
  .picker-label {
    font-size: 13px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .picker-hint {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--muted-foreground);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  b.inc {
    color: var(--success);
  }
  b.exc {
    color: var(--destructive);
  }
  .chev {
    font-size: 18px;
    color: var(--muted-foreground);
    flex-shrink: 0;
  }
  .picked {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 0 14px 14px;
    max-height: 120px;
    overflow-y: auto;
  }
  .chip {
    border: 1px solid var(--primary);
    border-radius: 10px;
    background: transparent;
    color: var(--primary);
    font-size: 13px;
    padding: 5px 12px;
    cursor: pointer;
  }
  .chip.inc {
    border-color: var(--success);
    color: var(--success);
  }
  .chip.exc {
    border-color: var(--destructive);
    color: var(--destructive);
  }
  .sheet-wrap {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .sheet-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    border: none;
    cursor: default;
    padding: 0;
  }
  .sheet {
    position: relative;
    width: min(480px, 92vw);
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    background: var(--background);
    border: 1px solid var(--border);
    border-radius: 16px;
    overflow: hidden;
  }
  .sheet-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 8px 4px 16px;
  }
  .sheet-head h2 {
    margin: 0;
    font-size: 16px;
  }
  .sheet-actions {
    display: flex;
    align-items: center;
  }
  .counts {
    margin: 0;
    padding: 0 16px 8px;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  .search {
    margin: 0 16px 4px;
    height: 40px;
    padding: 0 12px;
    border-radius: var(--radius);
    border: 1px solid var(--input);
    background: var(--muted);
    color: inherit;
    font-size: 14px;
  }
  .search:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 22%, transparent);
  }
  .sheet-body {
    overflow-y: auto;
    padding: 4px 16px 12px;
  }
  .sheet-body h3 {
    font-size: 12px;
    color: var(--muted-foreground);
    margin: 14px 0 8px;
  }
  .picks {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: transparent;
    color: inherit;
    font-size: 13px;
    padding: 5px 12px;
    cursor: pointer;
  }
  .pick:hover {
    border-color: var(--primary);
  }
  .pick.inc {
    border-color: var(--success);
    color: var(--success);
  }
  .pick.exc {
    border-color: var(--destructive);
    color: var(--destructive);
  }
  .avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 800;
  }
  .pick.inc .avatar {
    background: var(--success);
    color: var(--background);
  }
  .pick.exc .avatar {
    background: var(--destructive);
    color: var(--background);
  }
  .sheet-foot {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 12px 16px 16px;
    border-top: 1px solid var(--border);
  }
</style>
