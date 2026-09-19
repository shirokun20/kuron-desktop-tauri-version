<script lang="ts">
  // AboutPage — port `about_screen.dart` mobile ke popup window (#about).
  // Hero pulse + pill versi live, kartu pembaruan (GitHub releases desktop),
  // menu Komunitas & Info (akordeon: lisensi/legal/FAQ/donasi), chips stack, footer.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { marked } from "marked";
  import logoApp from "../../assets/icons/logo_app.webp";
  import qrisImg from "../../assets/donation_qris.jpeg";
  import termsMd from "../../assets/legal/id/terms_and_conditions.md?raw";
  import privacyMd from "../../assets/legal/id/privacy_policy.md?raw";
  import faqMd from "../../assets/legal/id/faq.md?raw";

  interface AppInfo {
    name: string;
    version: string;
    backend: string;
  }

  const REPO_MOBILE = "https://github.com/shirokun20/nhasixapp";
  const RELEASES_API =
    "https://api.github.com/repos/shirokun20/kuron-desktop-tauri-version/releases/latest";
  const SPONSORS = "https://github.com/sponsors/shirokun20";

  const DEPS = [
    { name: "Tauri", version: "2.11.5" },
    { name: "tauri-plugin-opener", version: "2.x" },
    { name: "serde / serde_json", version: "1.x" },
    { name: "@tauri-apps/api", version: "2.11.1" },
    { name: "@tauri-apps/plugin-opener", version: "2.5.5" },
    { name: "Svelte", version: "5.57.0" },
    { name: "Vite", version: "8.3.0" },
    { name: "TypeScript", version: "5.9.3" },
    { name: "marked", version: "17.x" },
  ];

  const CHIPS = ["Tauri v2", "Rust", "Svelte 5", "Clean Arch"];

  let info = $state<AppInfo>({ name: "Kuron Desktop", version: "…", backend: "tauri-v2" });
  let expanded = $state<string | null>(null);
  let updateStatus = $state("Periksa pembaruan");
  let updateUrl = $state<string | null>(null);
  let checking = $state(false);
  let legalHtml = $state<Record<string, string>>({});

  const svg = (inner: string) =>
    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
  const I = {
    refresh: svg('<path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/><path d="M3 21v-5h5"/>'),
    code: svg('<path d="m16 18 6-6-6-6"/><path d="m8 6-6 6 6 6"/>'),
    doc: svg('<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>'),
    gavel: svg('<path d="m14.5 12.5-8 8a2.119 2.119 0 1 1-3-3l8-8"/><path d="m16 16 6-6"/><path d="m8 8 6-6"/><path d="m9 7 8 8"/><path d="m21 11-8-8"/>'),
    shield: svg('<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1 1 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="M12 8v4"/><path d="M12 16h.01"/>'),
    help: svg('<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>'),
    coffee: svg('<path d="M17 8h1a4 4 0 1 1 0 8h-1"/><path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z"/><path d="M6 2v2M10 2v2M14 2v2"/>'),
    chev: svg('<path d="m9 18 6-6-6-6"/>'),
    ext: svg('<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>'),
    heart: svg('<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>'),
  };

  onMount(() => {
    invoke<AppInfo>("cmd_app_info")
      .then((r) => (info = r))
      .catch(() => {});
    Promise.all([termsMd, privacyMd, faqMd].map((m) => marked.parse(m))).then(
      ([t, p, f]) => {
        legalHtml = { terms: t as string, privacy: p as string, faq: f as string };
      },
    );
  });

  function toggle(id: string) {
    expanded = expanded === id ? null : id;
  }

  async function open(link: string) {
    try {
      await openUrl(link);
    } catch {
      window.open(link, "_blank");
    }
  }

  async function checkUpdate() {
    if (checking) return;
    if (updateUrl) {
      open(updateUrl);
      return;
    }
    checking = true;
    updateStatus = "Memeriksa…";
    try {
      const res = await fetch(RELEASES_API);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      const tag = String(data.tag_name ?? "").replace(/^v/, "");
      updateUrl = String(data.html_url ?? "");
      updateStatus = tag && tag !== info.version ? `Tersedia ${data.tag_name}` : "Sudah versi terbaru";
    } catch {
      updateStatus = "Gagal memeriksa";
    } finally {
      checking = false;
    }
  }
</script>

<main class="about">
  <!-- Hero ala mobile: logo pulse + judul + pill versi -->
  <div class="hero">
    <img class="logo" src={logoApp} alt="Logo Kuron" />
  </div>
  <h1>Kuron</h1>
  <p class="pill">v{info.version}</p>

  <!-- PEMBARUAN -->
  <h2 class="section">Pembaruan</h2>
  <button class="card update" onclick={checkUpdate} disabled={checking}>
    <span class="circle">{@html I.refresh}</span>
    <span class="texts">
      <strong>Pembaruan Aplikasi</strong>
      <small>{updateStatus}</small>
    </span>
    <span class="trail">{@html I.chev}</span>
  </button>

  <!-- KOMUNITAS & INFO -->
  <h2 class="section">Komunitas &amp; Info</h2>
  <div class="card menu">
    <button class="row" onclick={() => open(REPO_MOBILE)}>
      <span class="lead">{@html I.code}</span>
      <span class="texts"><strong>Repositori GitHub</strong><small>Lihat kode sumber &amp; berkontribusi</small></span>
      <span class="trail sm">{@html I.ext}</span>
    </button>
    <hr />
    <button class="row" onclick={() => toggle("licenses")}>
      <span class="lead">{@html I.doc}</span>
      <span class="texts"><strong>Lisensi Sumber Terbuka</strong><small>Pustaka yang digunakan di aplikasi ini</small></span>
      <span class="trail">{@html I.chev}</span>
    </button>
    {#if expanded === "licenses"}
      <ul class="deps">
        {#each DEPS as d}
          <li><span>{d.name}</span><small>{d.version}</small></li>
        {/each}
      </ul>
    {/if}
    <hr />
    <button class="row" onclick={() => toggle("terms")}>
      <span class="lead">{@html I.gavel}</span>
      <span class="texts"><strong>Syarat dan Ketentuan</strong><small>Perjanjian pengguna dan disclaimer</small></span>
      <span class="trail">{@html I.chev}</span>
    </button>
    {#if expanded === "terms"}
      <div class="md">{@html legalHtml.terms ?? "Memuat…"}</div>
    {/if}
    <hr />
    <button class="row" onclick={() => toggle("privacy")}>
      <span class="lead">{@html I.shield}</span>
      <span class="texts"><strong>Kebijakan Privasi</strong><small>Bagaimana kami menangani data Anda</small></span>
      <span class="trail">{@html I.chev}</span>
    </button>
    {#if expanded === "privacy"}
      <div class="md">{@html legalHtml.privacy ?? "Memuat…"}</div>
    {/if}
    <hr />
    <button class="row" onclick={() => toggle("faq")}>
      <span class="lead">{@html I.help}</span>
      <span class="texts"><strong>FAQ</strong><small>Pertanyaan yang sering diajukan</small></span>
      <span class="trail">{@html I.chev}</span>
    </button>
    {#if expanded === "faq"}
      <div class="md">{@html legalHtml.faq ?? "Memuat…"}</div>
    {/if}
    <hr />
    <button class="row" onclick={() => toggle("donate")}>
      <span class="lead">{@html I.coffee}</span>
      <span class="texts"><strong>Dukung Pengembang</strong><small>Traktir saya kopi</small></span>
      <span class="trail">{@html I.chev}</span>
    </button>
    {#if expanded === "donate"}
      <div class="donate">
        <p>Dukung pengembangan via QRIS. Terima kasih! ☕</p>
        <img class="qris" src={qrisImg} alt="QRIS donasi pengembang" />
        <button class="sponsors" onclick={() => open(SPONSORS)}>
          <span class="heart">{@html I.heart}</span> GitHub Sponsors
        </button>
        <p class="thanks">Terima kasih atas dukunganmu!</p>
      </div>
    {/if}
  </div>

  <!-- DIBANGUN DENGAN -->
  <h2 class="section">Dibangun Dengan</h2>
  <div class="chips">
    {#each CHIPS as c}
      <span class="chip">{c}</span>
    {/each}
  </div>

  <footer>
    <p>Dibuat dengan ♥ oleh Shirokun20</p>
    <p class="dim">© 2025 Hak Cipta Dilindungi</p>
  </footer>
</main>

<style>
  .about {
    min-height: 100vh;
    padding: 28px 24px 24px;
    background: var(--background);
  }
  .hero {
    display: flex;
    justify-content: center;
    margin: 8px 0 20px;
  }
  .logo {
    width: 120px;
    height: 120px;
    object-fit: contain;
    animation: pulse 2s ease-in-out infinite alternate;
  }
  @keyframes pulse {
    from { transform: scale(1); }
    to { transform: scale(1.1); }
  }
  h1 {
    margin: 0 0 8px;
    font-size: 32px;
    text-align: center;
  }
  .pill {
    width: fit-content;
    margin: 0 auto 28px;
    padding: 6px 12px;
    border-radius: 999px;
    background: var(--muted);
    font-size: 14px;
    font-weight: 700;
    color: var(--muted-foreground);
  }
  .section {
    margin: 0 0 8px 8px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--primary);
  }
  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 16px;
    margin-bottom: 24px;
    overflow: hidden;
  }
  .update {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px;
    cursor: pointer;
    border: 1px solid var(--border);
    text-align: left;
  }
  .update:disabled {
    cursor: wait;
    opacity: 0.8;
  }
  .circle {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: color-mix(in srgb, var(--primary) 18%, transparent);
    color: var(--primary);
  }
  .circle :global(svg) {
    width: 24px;
    height: 24px;
  }
  .update:disabled .circle :global(svg) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .texts {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .texts strong {
    font-size: 16px;
  }
  .texts small {
    font-size: 14px;
    color: var(--muted-foreground);
  }
  .trail {
    display: inline-flex;
    color: var(--muted-foreground);
    opacity: 0.6;
  }
  .trail :global(svg) {
    width: 22px;
    height: 22px;
  }
  .trail.sm :global(svg) {
    width: 16px;
    height: 16px;
  }
  .menu {
    padding: 4px 0;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
  }
  .row:hover {
    background: var(--muted);
  }
  .lead {
    display: inline-flex;
    color: var(--muted-foreground);
    flex-shrink: 0;
  }
  .lead :global(svg) {
    width: 24px;
    height: 24px;
  }
  .menu hr {
    margin: 0 16px;
    border: none;
    border-top: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .deps {
    list-style: none;
    margin: 0;
    padding: 4px 16px 12px 56px;
  }
  .deps li {
    display: flex;
    justify-content: space-between;
    padding: 6px 0;
    font-size: 13px;
    border-bottom: 1px dashed var(--border);
  }
  .deps li:last-child {
    border-bottom: none;
  }
  .deps small {
    color: var(--muted-foreground);
  }
  .md {
    padding: 4px 20px 16px 56px;
    font-size: 13px;
    line-height: 1.65;
    color: var(--muted-foreground);
  }
  .md :global(h1) {
    font-size: 16px;
    color: var(--foreground);
  }
  .md :global(h2), .md :global(h3) {
    font-size: 14px;
    color: var(--foreground);
  }
  .md :global(table) {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .md :global(th), .md :global(td) {
    border: 1px solid var(--border);
    padding: 4px 8px;
    text-align: left;
  }
  .md :global(blockquote) {
    margin: 8px 0;
    padding-left: 12px;
    border-left: 3px solid var(--primary);
  }
  .md :global(code) {
    font-size: 12px;
  }
  .md :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 12px 0;
  }
  .donate {
    padding: 4px 20px 16px 56px;
    font-size: 14px;
    color: var(--muted-foreground);
  }
  .qris {
    display: block;
    width: 200px;
    border-radius: 12px;
    border: 1px solid var(--border);
    margin-bottom: 16px;
  }
  .donate p {
    margin: 8px 0 12px;
  }
  .sponsors {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    border: none;
    border-radius: 999px;
    background: #ea4aaa;
    color: #fff;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
  }
  .heart {
    display: inline-flex;
  }
  .heart :global(svg) {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }
  .thanks {
    font-weight: 700;
    color: var(--foreground);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin: 8px 0 28px;
  }
  .chip {
    font-size: 12px;
    font-weight: 700;
    padding: 6px 14px;
    border-radius: 10px;
    color: var(--primary);
    background: color-mix(in srgb, var(--primary) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--primary) 30%, transparent);
  }
  footer {
    text-align: center;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  footer p {
    margin: 2px 0;
  }
  footer .dim {
    font-size: 11px;
    opacity: 0.6;
  }
</style>
