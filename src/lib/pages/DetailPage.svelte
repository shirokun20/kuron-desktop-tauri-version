<script lang="ts">
  // DetailPage — sampul bingkai tinta + judul display + lane bahasa chapter.
  // Lane 1:1 `chapter_language_presenter.dart` (>1 lane = tab, else flat).
  // Chapter internal → reader in-app; eksternal → browser via opener.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../api/client";
  import MainGridCard from "../components/MainGridCard.svelte";
  import type { Chapter, Comment, Content } from "../domain/types";
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

  let fetched = $state<Content | null>(null);
  let chapters = $state<Chapter[]>([]);
  let laneKey = $state<string | null>(null);
  let related = $state<Content[]>([]);
  let comments = $state<Comment[]>([]);
  let loading = $state(true);
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
  let fav = $derived(libraryStore.isFav(content.id));
  // Prop sebagai tampilan awal; hasil fetch menimpanya saat tiba.
  let detail = $derived(fetched ?? content);
  let lanes = $derived(buildChapterLanes(chapters, laneKey));
  let visible = $derived(selectedLaneChapters(lanes));

  $effect(() => {
    const c = content;
    loading = true;
    error = null;
    fetched = null;
    chapters = [];
    laneKey = null;
    related = [];
    comments = [];
    let cancelled = false;
    (async () => {
      try {
        const [d, ch] = await Promise.all([
          api.detail(c.id, c.source_id),
          api.chapters(c.id, c.source_id),
        ]);
        if (cancelled) return;
        fetched = d;
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
    onopenchapter(ch, visible);
  }
</script>

<section class="detail">
  <button class="ghost back" onclick={onback}>← Kembali</button>

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
      <p class="kicker">{detail.source_id}</p>
      <h1>{detail.title}</h1>
      <div class="badges">
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

  {#if detail.tags.length > 0}
    <div class="tags" aria-label="Tag">
      {#each detail.tags as tag (tag.id)}
        <span class="tag" title={tag.tag_type}>{tag.name}</span>
      {/each}
    </div>
  {/if}

  <div class="chapters-head">
    <h2>Bab</h2>
    {#if lanes.lanes.length > 1}
      <div class="lanes" role="tablist" aria-label="Bahasa bab">
        {#each lanes.lanes as lane (lane.key)}
          <button
            class="lane"
            class:on={lanes.selectedKey === lane.key}
            role="tab"
            aria-selected={lanes.selectedKey === lane.key}
            onclick={() => (laneKey = lane.key)}
          >
            {langFlag(lane.key)}
            {langLabel(lane.key)}
            <span class="count">{lane.chapters.length}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if loading}
    <p class="muted">Memuat bab…</p>
  {:else if visible.length === 0 && !error}
    <p class="muted">Belum ada bab untuk konten ini.</p>
  {:else}
    <ul class="rows">
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
  {/if}

  {#if related.length > 0}
    <h2 class="section">Terkait</h2>
    <div class="rail">
      {#each related as item (item.id)}
        <div class="rail-card">
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
</section>

<style>
  .detail {
    --ink-shadow: 5px 5px 0 rgb(0 0 0 / 0.35);
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
  .back {
    margin-bottom: 4px;
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
    margin: 12px 0 28px;
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
  .kicker {
    margin: 0;
    font-family: "Komika", system-ui, sans-serif;
    font-size: 13px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--primary);
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
  .lane .count {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 15px;
    letter-spacing: 0.06em;
    opacity: 0.85;
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
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 24px;
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
  .rail {
    display: flex;
    gap: 14px;
    overflow-x: auto;
    padding-bottom: 8px;
  }
  .rail-card {
    flex: 0 0 160px;
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
</style>
