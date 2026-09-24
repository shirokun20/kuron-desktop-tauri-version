<script lang="ts">
  // ReaderCanvas — 2 mode baca (keputusan user: horizontal tak worth it):
  // - paginated: 1 halaman + tombol/keyboard, fit pas viewport.
  // - vertical: scroll atas-bawah; tiap gambar fit LEBAR kolom penuh
  //   (screenshot user: space kosong di bawah = kolom lebih lebar dari
  //   gambar + contain menyisakan ruang). Gambar tinggi = scroll panjang
  //   alami. Sticky: yang pernah tampil tak dibongkar; gagal sesaat
  //   auto-retry 1×.
  import { cubicOut } from "svelte/easing";
  import { fly } from "svelte/transition";
  import type { ReaderMode } from "../stores/settingsPersist";
  import { paginate } from "../utils/readerLayout";

  const KEEP = 1;

  let {
    pages,
    mode,
    rtl = false,
    current,
    failed,
    onpage,
    onfail,
    onretry,
    onok,
  }: {
    pages: string[];
    mode: ReaderMode;
    rtl?: boolean;
    current: number;
    failed: Set<number>;
    onpage: (p: number) => void;
    onfail: (p: number) => void;
    onretry: (p: number) => void;
    /** Gambar sukses termuat → bersihkan flag gagal (pulih dari error sesaat). */
    onok: (p: number) => void;
  } = $props();

  let listEl: HTMLElement | null = $state(null);
  let total = $derived(pages.length);
  let near = $derived.by(() => {
    const out = new Set<number>();
    const c = Math.min(Math.max(1, current), Math.max(1, total));
    for (let p = c - KEEP; p <= c + KEEP; p++) {
      if (p >= 1 && p <= total) out.add(p);
    }
    return out;
  });
  /** Halaman yang pernah sukses tampil — tidak pernah dibongkar lagi. */
  let seen = $state(new Set<number>());
  /** Dimensi alami per halaman — cadangan tinggi placeholder anti-loncat. */
  let dims = $state(new Map<number, { w: number; h: number }>());
  /** Rasio w/h estimasi se-bab (median yang terukur; default portrait). */
  function estRatio(): number {
    if (dims.size === 0) return 0.7;
    const rs = [...dims.values()]
      .map((d) => d.w / d.h)
      .filter((r) => Number.isFinite(r) && r > 0)
      .sort((a, b) => a - b);
    if (rs.length === 0) return 0.7;
    return rs[Math.floor(rs.length / 2)];
  }
  /** Rasio kotak halaman: eksak bila terukur, estimasi bila belum. */
  function ratioFor(p: number): number {
    const d = dims.get(p);
    if (d) return d.w / d.h;
    return estRatio();
  }
  /** Hitung auto-retry per halaman (gagal sesaat → coba sekali lagi). */
  let ticks = $state(new Map<number, number>());
  /** Nonce remount per halaman — memaksa `<img>` rebuild agar retry
      pertama benar-benar fetch ulang (akar "tak pernah show": retry
      lama tanpa nonce = no-op, `failed` kosong, tile tak muncul). */
  let nonce = $state(new Map<number, number>());
  function bump(p: number) {
    nonce.set(p, (nonce.get(p) ?? 0) + 1);
    nonce = new Map(nonce);
  }
  /** Tampil bila dekat posisi ATAU pernah terlihat (sticky, anti-hilang). */
  function showable(p: number): boolean {
    return near.has(p) || seen.has(p);
  }
  function markSeen(p: number, w?: number, h?: number) {
    if (w && h && (!dims.get(p) || dims.get(p)?.w !== w)) {
      dims.set(p, { w, h });
      dims = new Map(dims);
    }
    if (!seen.has(p)) {
      seen.add(p);
      seen = new Set(seen);
    }
    onok(p);
  }
  /** Gagal sesaat (rate-limit/hotlink) → remount sekali; gagal 2× = error. */
  function markMiss(p: number) {
    const n = ticks.get(p) ?? 0;
    if (n < 1) {
      ticks.set(p, n + 1);
      ticks = new Map(ticks);
      window.setTimeout(() => {
        if (!seen.has(p)) {
          bump(p);
          onretry(p);
        }
      }, 900);
    } else {
      onfail(p);
    }
  }
  /** Retry manual per-halaman → remount paksa juga. */
  function manualRetry(p: number) {
    bump(p);
    onretry(p);
  }
  // Ganti bab → reset sesi. JANGAN reset `seen`: itu akar "scroll
  // malah hilang" — gambar yang sudah tampil dibongkar lalu fetch ulang.
  // `pages` baru = indeks 1-based menunjuk URL berbeda, jadi aman.
  $effect(() => {
    void pages;
    ticks = new Map();
    nonce = new Map();
  });

  function go(delta: 1 | -1) {
    onpage(paginate(current, total, delta, rtl));
  }
  /** Arah geser animasi: maju = dari kanan, mundur = dari kiri (+RTL). */
  let slideSign = $state(1);
  let lastPage = $state(1);
  $effect(() => {
    const c = current;
    if (c !== lastPage) {
      slideSign = (rtl ? -1 : 1) * (c > lastPage ? 1 : -1);
      lastPage = c;
    }
  });

  // Keyboard: panah + RTL, PgUp/Dn, Home/End. Vertical: PgDn/Spasi =
  // scroll mulus satu layar (bukan lompat halaman) agar terasa membaca.
  $effect(() => {
    const t = total;
    const m = mode;
    const h = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName?.toLowerCase();
      if (tag === "input" || tag === "textarea" || tag === "select") return;
      if (e.key === "ArrowRight") {
        e.preventDefault();
        go(1);
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        go(-1);
      } else if (e.key === "PageDown" || e.key === " ") {
        e.preventDefault();
        if (m === "vertical") smoothStep(1);
        else go(1);
      } else if (e.key === "PageUp") {
        e.preventDefault();
        if (m === "vertical") smoothStep(-1);
        else go(-1);
      } else if (e.key === "Home") {
        e.preventDefault();
        onpage(1);
      } else if (e.key === "End") {
        e.preventDefault();
        onpage(t);
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  });

  // Spy posisi vertikal = tengah viewport. Paginated tanpa spy.
  $effect(() => {
    const m = mode;
    if (!listEl || total === 0 || m === "paginated") return;
    const imgs = listEl.querySelectorAll("[data-vpage]");
    const spy = new IntersectionObserver(
      (entries) => {
        for (const en of entries) {
          if (!en.isIntersecting) continue;
          const p = Number((en.target as HTMLElement).dataset.vpage);
          if (Number.isInteger(p) && p !== current) onpage(p);
        }
      },
      { root: null, rootMargin: "-45% 0px -45% 0px", threshold: 0 },
    );
    imgs.forEach((el) => spy.observe(el));
    return () => spy.disconnect();
  });

  // Paginated ganti halaman → atas (efek terikat mode agar re-jalan).
  $effect(() => {
    if (mode === "paginated") listEl?.scrollTo?.({ top: 0 });
  });

  /** Scroll keyboard vertikal yang mulus (halaman penuh per tekan). */
  function smoothStep(delta: 1 | -1) {
    const scroller = document.querySelector(".canvas-wrap.scroll");
    if (scroller) {
      const h = scroller.clientHeight * 0.9;
      scroller.scrollBy({ top: delta * h, behavior: "smooth" });
      return;
    }
    go(delta);
  }
</script>

{#if mode === "paginated"}
  <div class="paged" bind:this={listEl}>
    {#if total > 0}
      {@const i = Math.min(total, Math.max(1, current)) - 1}
      {@const src = pages[i]}
      {#key i}
        <figure
          in:fly={{
            x: 56 * slideSign,
            duration: 220,
            easing: cubicOut,
          }}
        >
          {#if failed.has(i + 1)}
            <div class="page-fail">
              <span class="fail-num">{i + 1}</span>
              <p>Gagal muat halaman ini.</p>
              <button class="ghost" onclick={() => manualRetry(i + 1)}>
                ⟳ Muat ulang
              </button>
            </div>
          {:else}
            {#key `${i + 1}-${nonce.get(i + 1) ?? 0}`}
          <img
            class="fit"
            src={src}
            alt={`Halaman ${i + 1}`}
            draggable="false"
            referrerpolicy="no-referrer"
            onerror={() => markMiss(i + 1)}
            onload={(e) => {
              const el = e.currentTarget as HTMLImageElement;
              markSeen(i + 1, el.naturalWidth, el.naturalHeight);
            }}
          />
            {/key}
          {/if}
        </figure>
      {/key}
      <div class="paged-nav">
        <button class="ghost" disabled={current <= 1} onclick={() => go(-1)}>
          ← Sebelumnya
        </button>
        <span class="pos">{current} / {total}</span>
        <button
          class="ghost"
          disabled={current >= total}
          onclick={() => go(1)}
        >
          Berikutnya →
        </button>
      </div>
    {/if}
  </div>
{:else}
  <div class="vlist" bind:this={listEl}>
    {#each pages as src, i (i)}
      {@const p = i + 1}
      <figure>
        {#if failed.has(p)}
          <div class="page-fail" data-vpage={p}>
            <span class="fail-num">{p}</span>
            <p>Gagal muat halaman ini.</p>
            <button class="ghost" onclick={() => manualRetry(p)}>
              ⟳ Muat ulang
            </button>
          </div>
        {:else if !showable(p)}
          {@const r = ratioFor(p)}
          <div
            class="hold tall ratio"
            style:aspect-ratio={`${r}`}
            data-vpage={p}
            aria-label={`Halaman ${p}`}
          >
            <span>{p}</span>
          </div>
        {:else}
          {#key `${p}-${nonce.get(p) ?? 0}`}
            <img
              class="vimg"
              {src}
              data-vpage={p}
              alt={`Halaman ${p}`}
              loading="lazy"
              draggable="false"
              referrerpolicy="no-referrer"
              onerror={() => markMiss(p)}
              onload={(e) => {
                const el = e.currentTarget as HTMLImageElement;
                markSeen(p, el.naturalWidth, el.naturalHeight);
              }}
            />
          {/key}
        {/if}
      </figure>
    {/each}
  </div>
{/if}

<style>
  /* Paginated: isi penuh ruang kanvas (flex), bukan tebakan viewport.
     Gambar seheight mungkin dalam ruang sisa — tanpa space kosong. */
  .paged {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .paged figure {
    margin: 0;
    width: 100%;
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .paged img.fit {
    display: block;
    max-width: 100%;
    max-height: 100%;
    height: 100%;
    width: auto;
    object-fit: contain;
    background: #000;
    border-radius: 2px;
  }
  .paged-nav {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pos {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 15px;
    letter-spacing: 0.08em;
    color: var(--muted-foreground);
  }
  /* Vertikal: tiap gambar FIT LEBAR kolom penuh (width:100%), tinggi
     alami mengikuti rasio — kolom rapat tanpa celah. Akar "space di
     bawah" (screenshot): figure + contain di kanvas lebih tinggi dari
     gambar + footer mengalir jauh. Kini gap 2px, tanpa kanvas berlebih. */
  .vlist {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding-bottom: 4px;
  }
  .vlist figure {
    margin: 0;
    width: 100%;
    max-width: 900px;
    line-height: 0;
  }
  /* Mulus: tiap gambar fade-in saat pertama tampil; scroll bawaan
     browser sudah smooth via `scroll-behavior` di kanvas shell. */
  .vimg {
    display: block;
    width: 100%;
    height: auto;
    border-radius: 2px;
    background: var(--muted);
    animation: vfade 260ms ease;
  }

  @keyframes vfade {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .vlist .page-fail {
    max-width: 900px;
    min-height: 60vh;
  }
  .page-fail {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 280px;
    line-height: 1.4;
    width: 100%;
    max-width: 560px;
    border: 1px dashed var(--destructive);
    border-radius: 4px;
    background: color-mix(in srgb, var(--destructive) 7%, transparent);
    padding: 28px 16px;
    text-align: center;
  }
  .page-fail p {
    margin: 0;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  .fail-num {
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 44px;
    line-height: 1;
    color: var(--destructive);
    opacity: 0.7;
  }
  .ghost {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 13px;
    font-size: 14px;
    font-weight: 700;
    color: var(--foreground);
    cursor: pointer;
    flex-shrink: 0;
  }
  .ghost:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .hold {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 420px;
    line-height: 1.4;
    width: 100%;
    border-radius: 4px;
    background: var(--muted);
    color: var(--muted-foreground);
    font-family: "Bangers", system-ui, sans-serif;
    font-size: 28px;
  }
  .hold.tall {
    min-height: 70vh;
  }
  /* Anti-loncat: placeholder setinggi gambar asli (rasio terukur atau
     median se-bab). Spinner di tengah via shimmer. */
  .hold.ratio {
    aspect-ratio: 0.7;
    min-height: 0;
  }
  .hold.ratio span {
    animation: pulse 1.1s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 0.45;
    }
    50% {
      opacity: 1;
    }
  }
</style>
