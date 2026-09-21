<script lang="ts">
  // DetailPage — route khusus fullscreen: hero tinta + sinopsis + tag grup +
  // rel chapters di samping (20 preview + sheet penuh ala
  // `ChapterListBottomSheet` mobile) + terkait + komentar.
  // Chip bahasa 1:1 mobile: dari `available_languages` detail (bukan feed),
  // tap chip = fetch ulang per bahasa (`loadChapterLane`); sumber tanpa
  // daftar bahasa fallback ke lane hasil grouping feed.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../api/client";
  import MainGridCard from "../components/MainGridCard.svelte";
  import type { Chapter, Comment, Content, Tag } from "../domain/types";
  import { libraryStore } from "../stores/library.svelte";
  import {
    buildChapterLanes,
    selectedLaneChapters,
  } from "../utils/chapterLanes";
  import { langFlag, langLabel } from "../utils/lang";

  let {
    content,
    onback,
    onopenchapter,
    onselectcontent,
  }: {
    content: Content;
    onback: () => void;
    onopenchapter: (ch: Chapter, list: Chapter[]) => void;
    onselectcontent: (c: Content) => void;
  } = $props();

  /** Preview rail: 20 pertama, sisanya via sheet (mobile: bottom sheet). */
  const RAIL_PREVIEW = 20;
  /** Urutan grup tag: tipe umum dulu, sisanya alfabet. */
  const TAG_GROUP_ORDER = [
    "category",
    "genre",
    "theme",
    "format",
    "content",
    "artist",
    "character",
    "parody",
    "group",
    "language",
    "tag",
  ];

  let fetched = $state<Content | null>(null);
  let chapters = $state<Chapter[]>([]);
  let laneKey = $state<string | null>(null);
  let related = $state<Content[]>([]);
  let comments = $state<Comment[]>([]);
  let loading = $state(true);
  let chaptersLoading = $state(false);
  let sheetOpen = $state(false);
  let error = $state<string | null>(null);

  function fmtDate(epochSecs: bigint | number | null | undefined): string {
    if (epochSecs == null) return "";
    const d = new Date(Number(epochSecs) * 1000);
    if (Number.isNaN(d.getTime())) return "";
    return d.toLocaleDateString("id-ID", {
      day: "numeric",
      month: "short",
      year: "numeric",
    });
  }

  function fmtCompact(n: bigint | number): string {
    return new Intl.NumberFormat("id-ID", { notation: "compact" }).format(
      Number(n),
    );
  }

  let fav = $derived(libraryStore.isFav(content.id));
  // Prop sebagai tampilan awal; hasil fetch menimpanya saat tiba.
  let detail = $derived(fetched ?? content);
  // Chip bahasa dari detail (mobile); kosong → grouping feed (scraper).
  let langChips = $derived(
    detail.available_languages.length > 0
      ? detail.available_languages
      : [...new Set(chapters.map((c) => c.language).filter((l) => l != null))],
  );
  let lanes = $derived(buildChapterLanes(chapters, laneKey));
  let visible = $derived(selectedLaneChapters(lanes));
  let preview = $derived(visible.slice(0, RAIL_PREVIEW));
  let tagGroups = $derived.by(() => {
    const groups = new Map<string, Tag[]>();
    for (const t of detail.tags) {
      const list = groups.get(t.tag_type);
      if (list) list.push(t);
      else groups.set(t.tag_type, [t]);
    }
    return [...groups.entries()].sort(([a], [b]) => {
      const ia = TAG_GROUP_ORDER.indexOf(a);
      const ib = TAG_GROUP_ORDER.indexOf(b);
      if (ia >= 0 || ib >= 0) {
        if (ia < 0) return 1;
        if (ib < 0) return -1;
        return ia - ib;
      }
      return a.localeCompare(b);
    });
  });

  /** Bahasa awal ala mobile: `en` bila tersedia, else pertama. */
  function initialLang(langs: string[]): string | null {
    if (langs.length === 0) return null;
    return langs.includes("en") ? "en" : langs[0];
  }

  async function loadChapters(lang: string | null, cancelled: () => boolean) {
    chaptersLoading = true;
    try {
      const ch = await api.chapters(content.id, content.source_id, lang);
      if (cancelled()) return;
      chapters = [...ch].sort((a, b) => a.order - b.order);
    } finally {
      if (!cancelled()) chaptersLoading = false;
    }
  }

  function selectLang(lang: string) {
    if (lang === laneKey || chaptersLoading) return;
    laneKey = lang;
    loadChapters(lang, () => false).catch(() => {
      // gagal ganti bahasa = rail diam (bukan error merah)
    });
  }

  $effect(() => {
    const c = content;
    loading = true;
    error = null;
    fetched = null;
    chapters = [];
    laneKey = null;
    related = [];
    comments = [];
    sheetOpen = false;
    let cancelled = false;
    (async () => {
      try {
        // Detail dulu (chip bahasa datang darinya), lalu bab.
        const d = await api.detail(c.id, c.source_id);
        if (cancelled) return;
        fetched = d;
        const lang = initialLang(d.available_languages);
        laneKey = lang;
        const ch = await api.chapters(c.id, c.source_id, lang);
        if (cancelled) return;
        chapters = [...ch].sort((a, b) => a.order - b.order);
        // Ala detail cubit mobile: terkait + komentar paralel non-blocking
        // setelah detail tampil; gagal = seksi absen (bukan error merah).
        api
          .related(c.id, c.source_id)
          .then((r) => {
            if (!cancelled) related = r;
          })
          .catch(() => {});
        api
          .comments(c.id, c.source_id)
          .then((cm) => {
            if (!cancelled) comments = cm;
          })
          .catch(() => {});
      } catch (e) {
        if (!cancelled) error = `gagal muat detail: ${e}`;
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  async function openChapter(ch: Chapter) {
    // Eksternal (baca di situs asal) → browser; sisanya reader in-app.
    if (ch.is_external) {
      if (ch.external_url) await openUrl(ch.external_url);
      return;
    }
    sheetOpen = false;
    onopenchapter(ch, visible);
  }

  function onSheetKey(e: KeyboardEvent) {
    if (e.key === "Escape") sheetOpen = false;
  }
</script>

<svelte:window onkeydown={sheetOpen ? onSheetKey : null} />

<section class="detail">
  <div class="topbar">
    <button class="ghost back" onclick={onback}>← Kembali</button>
    <span class="crumb">{detail.source_id}</span>
  </div>

  {#if error}
    <p class="err">{error}</p>
  {/if}

  <div class="hero halftone">
    <div class="cover-wrap">
      {#if detail.cover_url}
        <img
          class="cover"
          src={detail.cover_url}
          alt=""
          draggable="false"
          referrerpolicy="no-referrer"
        />
      {:else}
        <span class="cover cover-fallback">
          {detail.title.slice(0, 2).toUpperCase()}
        </span>
      {/if}
    </div>
    <div class="info">
      <h1>{detail.title}</h1>
      <div class="badges">
        {#if detail.rating != null}
          <span class="badge hot">★ {detail.rating.toFixed(1)}</span>
        {/if}
        {#if detail.favorites != null}
          <span class="badge">♥ {fmtCompact(detail.favorites)}</span>
        {/if}
        {#if detail.page_count}
          <span class="badge">{detail.page_count} hal</span>
        {/if}
        {#if detail.language}
          <span class="badge">
            {langFlag(detail.language)} {langLabel(detail.language)}
          </span>
        {/if}
        {#if detail.upload_date}
          <span class="badge">{detail.upload_date}</span>
        {/if}
        {#if chapters.length > 0}
          <span class="badge chapters">{chapters.length} bab</span>
        {/if}
      </div>
      <div class="actions">
        <button
          class="primary"
          disabled={visible.length === 0}
          onclick={() => visible.length > 0 && openChapter(visible[0])}
        >
          Mulai Baca
        </button>
        <button class="ghost" onclick={() => libraryStore.toggleFav(detail)}>
          {fav ? "♥ Favorit" : "♡ Favorit"}
        </button>
      </div>
    </div>
  </div>

  <div class="body">
    <div class="main-col">
      {#if detail.description}
        <h2 class="section">Sinopsis</h2>
        <p class="synopsis">{detail.description}</p>
      {/if}

      {#if tagGroups.length > 0}
        <h2 class="section">Tag</h2>
        <div class="tag-groups">
          {#each tagGroups as [group, tags] (group)}
            <div class="tag-group">
              <span class="tag-group-label">{group}</span>
              <div class="tags">
                {#each tags as tag (tag.id)}
                  <span
                    class="tag"
                    title={tag.count > 0n ? `${tag.count}×` : group}
                  >
                    {tag.name}
                  </span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if related.length > 0}
        <h2 class="section">Terkait</h2>
        <div class="rail">
          {#each related as item (item.id)}
            <div class="related-card">
              <MainGridCard content={item} onselect={onselectcontent} />
            </div>
          {/each}
        </div>
      {/if}

      {#if comments.length > 0}
        <h2 class="section">Komentar ({comments.length})</h2>
        <ul class="comments">
          {#each comments as cm (cm.id)}
            <li class="comment">
              {#if cm.avatar_url}
                <img
                  class="avatar"
                  src={cm.avatar_url}
                  alt=""
                  loading="lazy"
                  draggable="false"
                  referrerpolicy="no-referrer"
                />
              {:else}
                <span class="avatar fallback">
                  {(cm.username || "?").slice(0, 1).toUpperCase()}
                </span>
              {/if}
              <div class="cbody">
                <div class="chead">
                  <strong>{cm.username || "anon"}</strong>
                  {#if fmtDate(cm.post_date)}
                    <span class="muted">{fmtDate(cm.post_date)}</span>
                  {/if}
                </div>
                <p>{cm.body}</p>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <aside class="rail-col" aria-label="Daftar bab">
      <div class="chapters-card">
        <div class="chapters-head">
          <h2>Bab</h2>
          {#if langChips.length > 1}
            <div class="lanes" role="tablist" aria-label="Bahasa bab">
              {#each langChips as lang (lang)}
                <button
                  class="lane"
                  class:on={lanes.selectedKey === lang}
                  role="tab"
                  aria-selected={lanes.selectedKey === lang}
                  onclick={() => selectLang(lang)}
                >
                  {langFlag(lang)}
                  {langLabel(lang)}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        {#if loading || chaptersLoading}
          <p class="muted">Memuat bab…</p>
        {:else if visible.length === 0 && !error}
          <p class="muted">Belum ada bab untuk konten ini.</p>
        {:else}
          <ul class="rows">
            {#each preview as ch (ch.id)}
              <li>
                <button class="row" onclick={() => openChapter(ch)}>
                  <span class="num">{ch.order}</span>
                  <span class="title">{ch.title || `Bab ${ch.order}`}</span>
                  {#if ch.is_external}
                    <span class="ext">situs ↗</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
          {#if visible.length > RAIL_PREVIEW}
            <button class="ghost wide" onclick={() => (sheetOpen = true)}>
              Lihat semua {visible.length} bab
            </button>
          {/if}
        {/if}
      </div>
    </aside>
  </div>
</section>

{#if sheetOpen}
  <div
    class="sheet-backdrop"
    role="presentation"
    onclick={() => (sheetOpen = false)}
  ></div>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Semua bab">
    <div class="sheet-head">
      <h2>Semua bab ({visible.length})</h2>
      <button
        class="ghost"
        aria-label="Tutup daftar bab"
        onclick={() => (sheetOpen = false)}
      >
        ✕
      </button>
    </div>
    {#if langChips.length > 1}
      <div class="lanes sheet-lanes" role="tablist" aria-label="Bahasa bab">
        {#each langChips as lang (lang)}
          <button
            class="lane"
            class:on={lanes.selectedKey === lang}
            role="tab"
            aria-selected={lanes.selectedKey === lang}
            onclick={() => selectLang(lang)}
          >
            {langFlag(lang)}
            {langLabel(lang)}
          </button>
        {/each}
      </div>
    {/if}
    <ul class="rows sheet-rows">
      {#each visible as ch (ch.id)}
        <li>
          <button class="row" onclick={() => openChapter(ch)}>
            <span class="num">{ch.order}</span>
            <span class="title">{ch.title || `Bab ${ch.order}`}</span>
            {#if ch.is_external}
              <span class="ext">baca di situs ↗</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .detail {
    --ink-shadow: 5px 5px 0 rgb(0 0 0 / 0.35);
    height: 100vh;
    overflow-y: auto;
    padding: 20px 28px 48px;
    max-width: 1180px;
    margin: 0 auto;
  }
  .muted {
    color: var(--muted-foreground);
  }
  .err {
    color: var(--destructive);
  }
  .ghost {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 14px;
    font-size: 13px;
    font-weight: 600;
    color: var(--foreground);
    cursor: pointer;
  }
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 4px;
  }
  .crumb {
    font-family: "Komika", system-ui, sans-serif;
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--primary);
  }
  /* Motif khas Kuron Ink: titik halftone ala cetakan manga. */
  .halftone {
    background-image: radial-gradient(
      color-mix(in srgb, var(--foreground) 13%, transparent) 1.1px,
      transparent 1.3px
    );
    background-size: 12px 12px;
  }
  .hero {
    display: flex;
    gap: 24px;
    margin: 12px 0 24px;
    padding: 24px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background-color: var(--card);
  }
  .cover-wrap {
    flex-shrink: 0;
  }
  .cover {
    display: block;
    width: 172px;
    height: 230px;
    border-radius: 6px;
    object-fit: cover;
    background: var(--muted);
    border: 1px solid var(--border);
    box-shadow: var(--ink-shadow);
  }
  .cover-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 40px;
    font-weight: 800;
    color: var(--muted-foreground);
  }
  .info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
  }
  .info h1 {
    margin: 0;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 40px;
    font-weight: 400;
    line-height: 1.05;
    letter-spacing: 0.02em;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .badge {
    font-size: 11px;
    font-weight: 700;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--muted);
    color: var(--muted-foreground);
  }
  .badge.hot {
    background: color-mix(in srgb, var(--primary) 22%, var(--muted));
    color: var(--primary);
  }
  .badge.chapters {
    background: var(--primary);
    color: var(--primary-foreground);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 6px;
  }
  .primary {
    background: var(--primary);
    border: 1px solid var(--primary);
    border-radius: 8px;
    padding: 9px 20px;
    font-size: 14px;
    font-weight: 800;
    color: var(--primary-foreground);
    cursor: pointer;
    box-shadow: 3px 3px 0 rgb(0 0 0 / 0.35);
  }
  .primary:active {
    transform: translate(2px, 2px);
    box-shadow: 1px 1px 0 rgb(0 0 0 / 0.35);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: default;
    box-shadow: none;
  }
  /* Kolom ganda desktop: info kiri, rail bab kanan (sticky). */
  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 24px;
    align-items: start;
  }
  .chapters-card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 18px;
    position: sticky;
    top: 20px;
    max-height: calc(100vh - 40px);
    overflow-y: auto;
  }
  .chapters-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 14px;
  }
  .chapters-head h2 {
    margin: 0;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 26px;
    font-weight: 400;
    letter-spacing: 0.04em;
  }
  .lanes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .lane {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 6px 13px;
    font-size: 13px;
    font-weight: 600;
    color: var(--foreground);
    cursor: pointer;
  }
  .lane.on {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 16px 10px 14px;
    color: var(--foreground);
    cursor: pointer;
    text-align: left;
  }
  .chapters-card .row {
    background: var(--background);
  }
  .row:hover {
    border-color: var(--primary);
  }
  .row .num {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 24px;
    color: var(--primary);
    min-width: 40px;
    text-align: right;
    line-height: 1;
  }
  .row .title {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row .ext {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 700;
    color: var(--muted-foreground);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 3px 10px;
  }
  .ghost.wide {
    width: 100%;
    margin-top: 12px;
  }
  .synopsis {
    margin: 0 0 8px;
    font-size: 14px;
    line-height: 1.65;
    color: var(--foreground);
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .tag-groups {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 8px;
  }
  .tag-group {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .tag-group-label {
    flex-shrink: 0;
    min-width: 74px;
    font-family: "Komika", system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag {
    font-size: 12px;
    font-weight: 600;
    padding: 4px 12px;
    border-radius: 6px;
    background: var(--muted);
    color: var(--muted-foreground);
  }
  h2.section {
    margin: 28px 0 14px;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 26px;
    font-weight: 400;
    letter-spacing: 0.04em;
  }
  .main-col > h2.section:first-child {
    margin-top: 0;
  }
  .rail {
    display: flex;
    gap: 14px;
    overflow-x: auto;
    padding-bottom: 8px;
  }
  .rail .related-card {
    flex: 0 0 160px;
    position: static;
    max-height: none;
    overflow: visible;
    padding: 0;
    background: transparent;
    border: 0;
  }
  .comments {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .comment {
    display: flex;
    gap: 12px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px 14px;
  }
  .avatar {
    width: 34px;
    height: 34px;
    border-radius: 999px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--muted);
  }
  .avatar.fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 15px;
    font-weight: 800;
    color: var(--primary);
  }
  .cbody {
    min-width: 0;
    flex: 1;
  }
  .chead {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 13px;
  }
  .chead .muted {
    font-size: 12px;
  }
  .cbody p {
    margin: 4px 0 0;
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  /* Sheet daftar penuh: panel kanan (desktop) / bottom sheet (sempit). */
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgb(0 0 0 / 0.55);
  }
  .sheet {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 71;
    width: min(480px, 92vw);
    background: var(--background);
    border-left: 1px solid var(--border);
    box-shadow: -12px 0 40px rgb(0 0 0 / 0.4);
    display: flex;
    flex-direction: column;
    padding: 20px;
    gap: 14px;
  }
  .sheet-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .sheet-head h2 {
    margin: 0;
    font-family: "Bangers", "Komika", system-ui, sans-serif;
    font-size: 26px;
    font-weight: 400;
    letter-spacing: 0.04em;
  }
  .sheet-rows {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  /* Layar sempit: rail bab tepat di bawah hero, sheet dari bawah. */
  @media (max-width: 900px) {
    .detail {
      padding: 14px 14px 40px;
    }
    .hero {
      flex-direction: column;
      align-items: center;
      text-align: center;
    }
    .info {
      align-items: center;
    }
    .badges,
    .actions {
      justify-content: center;
    }
    .body {
      grid-template-columns: 1fr;
    }
    .rail-col {
      order: -1;
    }
    .chapters-card {
      position: static;
      max-height: none;
    }
    .sheet {
      top: auto;
      left: 0;
      right: 0;
      bottom: 0;
      width: auto;
      max-height: 82vh;
      border-left: 0;
      border-top: 1px solid var(--border);
      border-radius: 16px 16px 0 0;
      box-shadow: 0 -12px 40px rgb(0 0 0 / 0.4);
    }
  }
</style>
