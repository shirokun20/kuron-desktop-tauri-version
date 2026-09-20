# Kuron Desktop — Project Memory

> **Unified context file** for tracking progress across AI tools.
> Read by: **Codex** | **OpenCode** | **Manual Review**

---

## Project Identity

| Key | Value |
|---|---|
| **App Name** | **Kuron Desktop** |
| **Repo** | `kuron-desktop` (lokal, belum push) |
| **Platform** | Pure desktop win+mac+linux GUI (Tauri v2) |
| **Stack** | Rust + TypeScript + Vite 8 + Svelte 5.57 |
| **Version** | 0.1.0 (`id.nhasix.kuron`) |
| **Architecture** | Clean Architecture (Domain → Data → Presentation via IPC) |
| **State Management** | Svelte 5 runes stores (`*.svelte.ts`), ganti Cubit |
| **DI** | `tauri::Builder::manage(AppState)` (`Arc<dyn ContentRepository>`), ganti GetIt |
| **Networking** | `HttpClientManager` reqwest rustls + cookie jar + DoH JSON + UA/rate-limit/retry per config |
| **Routing** | `lib/router/route.svelte.ts` (`splash` \| `main`), SvelteKit Fase 4 |
| **Database** | `rusqlite` bundled (contents/chapters/downloads/translation_cache/history/favorites) + KV JSON + keychain |
| **Logging** | `tracing` subscriber (`RUST_LOG`, default info; tanpa log prompt/gambar) |
| **Sources** | Bundel HANYA nhentai; sisanya install via manifest-link/zip `kuron-extensions` (`ExtensionManager` + sha256) |

### Related Repos
| Repo | Role |
|---|---|
| `shirokun20/kuron-mobile` | Ibu: assets, DESIGN.md, pola UI (splash/main/cards) |
| `KURON_TAURI_V2_DESKTOP.md` | Spec arsitektur → `docs/architecture.md` |

### Key Features
- Splash → Main flow (init appInfo + feed saat splash, window resize 480×700 → 1200×800)
- App-shell NeedMCP (sidebar 256 + header 64 + konten 1180; tab sumber dihapus, sumber via sidebar)
- Material rasa Kuron: Light/Dark solid flat-hairline (dial Linen Meridian), tag palette 12 kategori
- Backend real Fase 2: network, SQLite/KV/keychain, cache, adapter scraper/REST + NHentai API live
- Ekstensi installable 1:1 mobile (bundel nhentai-only, manifest + sha256 + zip)
- Offline-first + AI translate reader (roadmap Fase 5/6, reuse Rust mobile)

---

## Architecture Overview

```
src-tauri/src/
├── core/              # AppError (Network/Storage), AppState DI Arc<dyn>, config, constants, logger(tracing)
├── domain/            # Pure Rust — serde + ts-rs (ADR-004); entities, repo traits async, value objects, services
│   ├── entities/      # hello, content, chapter, page_image_result, download_task,
│   │                  # ai_translation, glossary, search_filter, reader_settings (+roundtrip tests)
│   ├── repositories/  # ContentRepository/HomeFeedRepository async + blanket Arc
│   ├── value_objects/ # SourceId, Language
│   ├── services/      # title_parser
│   └── ts_export.rs   # test generate `src/lib/domain/types.ts` (15 tipe)
├── application/       # UseCases 1 file 1 execute async (say_hello, get_home_feed, search,
│   │                  # get_content_detail, get_chapters, get_page_images) + services/rate_limiter
├── data/              # MockContentRepository + ContentRepositoryImpl (remote-first)
│   ├── models/        # ContentModel (From Entity)
│   ├── datasources/   # config (SourceFile/overlay) + extension (manifest/sha256/zip)
│   │                  # + local (rusqlite/KV/keychain/file) + remote (scraper/REST/NHentai API)
│   ├── repositories/  # mock + real impl
│   └── native/        # image_ops stub (port kuron_native/rust Fase 3)
├── commands/          # Thin IPC: hello_world, app_info, home_feed, search, get_detail,
│   │                  # get_chapters, get_page_images
├── network.rs         # HttpClientManager (rustls/jar/retry) + DoH JSON + kuron_user_agent
├── cache.rs           # ImageCache (`$sourceId/$contentId/page_N.jpg` via FileCacheDs)
└── resources/         # source-configs/nhentai-config.json (SATU-SATUNYA bundel)

src/lib/
├── api/               # client.ts (typed invoke) + window.ts (resize + openPopup generik)
├── domain/            # types.ts GENERATED ts-rs (jangan edit manual)
├── stores/            # hello, content, source, theme (runes, ganti Cubit/Bloc)
├── router/            # route.svelte.ts, SvelteKit Fase 4
├── theme/             # tokens.ts (port colors/design_tokens/tag palette 1:1)
├── components/        # Sidebar, SourceList, TagChip, MainGridCard, MainFeaturedCard, ThemeToggle
└── pages/             # SplashScreen, MainPage, SourcePickerPage, AboutPage
```

### Layer Rules
- **Domain**: Pure Rust + `serde` + `ts-rs` (ADR-004). No Tauri/SQLite/reqwest.
- **Data**: Depends only on Domain. Remote/local real (mock hanya default AppState).
- **Application**: Depends only on Domain. Satu use case = satu file + `execute` async.
- **Presentation**: Svelte via `invoke('cmd_*')` only. Commands thin, `AppError -> String` di boundary.
- **Sources**: JSON config mobile (bundel nhentai-only, sisanya installable + sha256).

---

## Current Progress Dashboard

> Tracked via `openspec/` — Last updated: 2026-09-20 (20/37 tasks, change `kuron-desktop-roadmap` in-progress)

### Archived (in `openspec/changes/archive/`)
- *(none)*

### Active Changes (in `openspec/changes/`)
- `kuron-desktop-roadmap` — proposal + 13 spec + design + tasks (20/37 DONE: grup 1–3, 4.1–4.6)

### Open Issues (tech debt tercatat, sadar)
- `MockContentRepository` masih default AppState — wiring real (installed) pending 4.7/Fase 6
- `SOURCES`/`SOURCE_META` hardcode — daftar dinamis pending 4.7
- HTML NHentai/Hitomi/EH live 403 CF-edge — cookie-harvest pending (API v2 jadi jalur NHentai)
- CI `ci.yml` menunggu push pertama untuk hijau di 3 runner
- `KosugiMaru.ttf` 3.5MB di bundle — subset/lazy-load Fase 8
- SPA + route store manual — SvelteKit Fase 4
- Signing/notarize + updater — Fase 8

---

### Recent Sessions

> Session log in this table. Last updated: 2026-09-20.

| 2026-09-20 | OpenCode | Port paket generik mobile | Done | Dari `kuron_generic`: rantai image-fallback (data-src/data-lazy-src/pagespeed + placeholder `*_result`/pagespeed_static) + regex dotAll group1-else-0; `fill_url` (encode query, drop param kosong ala UrlBuilder — cegah 400 sekelas nhentai); rate-resolve minDelay→rps→rpm ala factory. Factory dedicated→generik + pipeline/referer dicatat untuk Fase 5/6. `cargo test` 36 pass/6 ignored, live NHentai+MangaDex hijau. |
| 2026-09-20 | OpenCode | Zip via file picker | Done | Link saja kurang → `tauri-plugin-dialog` + `cmd_extension_install_zip_file` (filter .zip, batal = info) + tombol "Pilih file .zip…" (URL tetap ada). Perlu restart `pnpm dev` (plugin + capability). Hijau semua. |
| 2026-09-20 | OpenCode | Badge config-driven + dialog fix | Done | Benar: badge hardcode. Kini `pageCount`+`language` dibaca dari fields config (ala mapper mobile; fallback defaultLanguage). Dialog zip: `blocking_pick_file` macetkan runtime → loading abadi; ganti callback + oneshot. Test fixture hijau. `cargo` 37 pass. Restart `pnpm dev` (command berubah). |
| 2026-09-20 | OpenCode | Fix meta kebaca sumber | Done | `load_dir` telan semua JSON → `ehentai-meta`/`mangadex-meta` muncul sebagai sumber. Kini hanya `*-config.json`; test regresi hijau. `cargo` 37 pass. Catatan: ikon HI/EH/MD lama belum ada (install sebelum fitur ikon) → install ulang sekali. Restart `pnpm dev` (Rust). |
| 2026-09-20 | OpenCode | Badge bahasa ganti badge sumber | Done | `Content.language` (ISO): NHentai dari tag_ids/detail-tags (12227→en, 29963→zh, default ja — `languageTagMap` mobile), situs JSON dari `defaultLanguage` (`normalize_lang`). Kartu: badge sumber → chip 🇬🇧 English dkk (`utils/lang`); featured meta ikut. `cargo` 36 pass, `pnpm` 0/0. |
| 2026-09-20 | OpenCode | Pagination + banner + kartu mobile | Done | Pagination ujung-ke-ujung: `home_feed(page)` di trait→mock→impl (mock hal>1 kosong), `home_page_path` pola `homePage`, `cmd_home_feed(source?, page?)`, store `load/loadMore` + dedupe, tombol "Muat lebih banyak". Featured → banner putar 5 terbaru (backdrop blur, dots, auto 6 dtk). Kartu ala mobile: cover full-bleed + scrim + badge halaman (`page_count` NHentai) + badge sumber. Blacklist belum (butuh settings Fase 7). `cargo` 36 pass, `pnpm` 0/0. |
| 2026-09-20 | OpenCode | Fix each_key_duplicate | Done | Feed areakomik isi slug sama 2× (seksi homepage beda) → `each (id)` crash → splash beku 100% (render main gagal). Dedupe by-id di `contentStore.load`; `svelte:boundary` di App dipertahankan + pesan ramah. Hijau 0/0, HMR cukup. |
| 2026-09-20 | OpenCode | Fix stuck splash 100% | Done | `await setMainSize()` di timer akhir: reject/hang IPC = `go("main")` ke-skip. Kini resize fire-and-forget + navigasi sinkron dijamin; `await current()` masuk try. Hijau 0/0, HMR cukup (frontend saja). |
| 2026-09-20 | OpenCode | Fix home feed scraper (areakomik) | Done | Akar: home_feed pakai pola search + query kosong → halaman "Hasil Pencarian" nol kartu (curl bukti). Kini `home_path` dari pola `home` mobile (`home_url()`, fallback list). Areakomik install→feed LIVE hijau. Restart `pnpm dev`. |
| 2026-09-20 | OpenCode | Cover + ikon tampil | Done | Ternyata kartu tak pernah render `<img>` (placeholder huruf pra-Fase 2). Kini: cover live + fallback huruf (`onerror`, lazy, `no-referrer` anti hotlink-guard); tile sumber pakai ikon manifest (`icon_url` disimpan saat install + `InstalledSource`, fallback inisial); teks "mock Fase 0" basi dihapus. Hijau 0/0. Catatan: ekstensi yang sudah terpasang sebelum ini belum punya ikon → install ulang sekali. |
| 2026-09-20 | OpenCode | Engine JSON generik (fix areakomik) | Done | `repo_for` kini baca pola `scraper` JSON ter-install (`source_config_from_json`: container/fields/regex/slug/inherits/detail/chapters/reader) — bukan cuma hardcode 4 situs. Engine dirombak ke `FieldMap` + `extract/extract_self` + `detail_title/cover`. Fixture areakomik (list/detail/chapters/pages) hijau; live areakomik tak terjangkau dari network ini (koneksi 000) — sumber CF-blocked kini warn-log, bukan silent. `cargo test` 33 pass/6 ignored. Restart `pnpm dev`. |
| 2026-09-20 | OpenCode | Fix feed nhentai 400 | Done | API tolak `query=` kosong → `search("")` pakai endpoint `allGalleries` (`/api/v2/galleries?page=`) dari config. Regresi di live test hijau. `cargo test` 32 pass. Restart `pnpm dev` bila sedang jalan (kode Rust). |
| 2026-09-20 | OpenCode | Apply 4.7 ekstensi UI + dinamis | Done | Popup `#extensions` (manifest URL + install/uninstall + zip) + item sidebar; store id-based + migrasi; 5 command baru; `cmd_home_feed(source?)` → `repo_for` (nhentai/mangadex/hitomi/eh via installed, "semua" mock); live manifest install hijau. `cargo test` 32 pass/5 ignored, `pnpm` 0/0 + build OK. Perlu restart `pnpm dev` (capability + command baru). tasks 21/37. |
| 2026-09-20 | OpenCode | Bundel nhentai-only + zip | Done | Klarifikasi: bundel HANYA nhentai (1:1 mobile), sisanya link/zip. `load_overlay` (installed menang), `install_zip_url/bytes` two-pass anti zip-slip. `cargo test` 30 pass/4 ignored, 0 warning, `pnpm` 0/0 + build OK. |
| 2026-09-20 | OpenCode | Pivot 4.6 ekstensi installable | Done | Tanpa bundel di app. `ExtensionManager`: fetch manifest resmi, install JSON + sha256 + tolak traversal, uninstall, `load_installed`; test hijau. tasks 20/37. |
| 2026-09-20 | OpenCode | Apply 4.6 config-driven | Superseded | Sempat bundel 45 JSON + parser + NHentai LIVE hijau, lalu dibatalkan (pivot installable). Engine dipertahankan. |
| 2026-09-20 | OpenCode | Apply grup 4 data real | Done | 4.1 reqwest rustls+jar+DoH CF (UA/cookie test hijau, DoH ignored); 4.2 rusqlite CRUD+migrasi hijau (plugin-sql ditolak: by-pass Application); 4.3 KV JSON + keyring real (test ignored); 4.4 FileCache cap+eviksi + ImageCache layout hijau; 4.5 engine scraper/REST + 3 site config + ContentRepositoryImpl + 4 usecase + 4 command; MangaDex LIVE hijau, NHentai HTML 403 CF-edge pending. `cargo test` 21 pass/3 ignored. tasks 19/35. |
| 2026-09-20 | OpenCode | Tentang resize paksa | Done | Fokus popup lama cuma `setFocus` (ukuran lama kekunci) → tambah `setSize` di `openPopup` + branch menu Rust. Hijau semua. |
| 2026-09-20 | OpenCode | Tentang width 560 | Done | Popup about 480×800 → 560×800 (`lib.rs` + `openAboutWindow`). Perlu restart `pnpm dev`. cargo check bersih. |
| 2026-09-20 | OpenCode | Hapus tab sumber konten | Done | Tab Semua/NHentai/Hitomi/E-Hentai + CSS `.tabs` dihapus dari MainPage; sumber cukup dari sidebar (popup). `pnpm check` 0/0, build OK. Spec `app-shell` diselaraskan. |
| 2026-09-20 | OpenCode | Apply grup 2+3 roadmap | Done | 2.1 tracing subscriber + test; 2.2 AppState `Arc<dyn ContentRepository>` + `cmd_home_feed(State)` + blanket Arc; 2.3 `.github/workflows/ci.yml` matrix 3 OS (YAML valid, hijau runner nunggu push); 3.1 roundtrip serde 4 test; 3.2 ts-rs 12 derive 15 tipe + `export_types_ts` tulis `types.ts` (`type`, bukan `interface`); 3.3 async-trait + command async. `cargo test` 11 pass, `cargo check` bersih, `pnpm check` 0/0, build OK. AGENTS domain rule → serde+ts-rs. tasks 14/35. |
| 2026-09-20 | OpenCode | Openspec payung roadmap | Done | Change `kuron-desktop-roadmap`: proposal + 13 spec delta + design + tasks. `validate` valid, 4/4 artefak. Planning only, tanpa ubah kode. |
| 2026-09-19 | OpenCode | Sidebar mini logo | Done | Collapsed 64px tambah logo maskot `logo_app.webp` 40px rounded+hairline di atas (`mini-logo` button, hover ring coral); klik → expand via `onToggle` baru (MainPage). HMR cukup. Hijau 0/0. |
| 2026-09-19 | OpenCode | Sidebar Tentang → popup | Done | Item `Tentang` (MORE) tadinya dead-end → buka popup `about` 480×800 via `openAboutWindow()` baru; `openPopup()` helper generik dipakai source-picker + about; `activeNav` tidak berubah, error tampil di konten. HMR cukup. Hijau 0/0. |
| 2026-09-19 | OpenCode | Tentang custom + menu macOS | Done | About panel native cuma nama+versi → window popup custom 460×640 (`AboutPage`, hash `#about`): logo, versi live `cmd_app_info`, deskripsi, stack chips, daftar sumber+versi, tombol repo (plugin-opener), footer. Menu `Tentang Kuron` custom via `WebviewWindowBuilder` (fokus bila ada). Cargo description/authors diperbaiki. Capability windows +`about`. cargo + svelte + build hijau. |
| 2026-09-19 | OpenCode | About port mobile 1:1 | Done | `AboutPage` ditulis ulang ikut `about_screen.dart`: hero `logo_app.webp` pulse + pill `v{live}`, PEMBARUAN (cek `releases/latest` repo desktop beneran, status + buka halaman rilis), KOMUNITAS & INFO 6 row + akordeon (lisensi = daftar dep nyata, S&K/Privasi/FAQ = md `assets/legal/id/` render `marked`, donasi = QRIS 151KB + GitHub Sponsors), DIBANGUN DENGAN (Tauri v2/Rust/Svelte 5/Clean Arch), footer Shirokun20 verbatim. Legal diadaptasi desktop (sumber NHentai/Hitomi/E-Hentai/MangaDex, path Pengaturan mobile dihapus, kontak repo desktop). Window 480×800. Hijau semua. |
| 2026-09-19 | OpenCode | MEMORY format mirror kuron-mobile | Done | Struktur disamakan: Identity + Related Repos + Key Features, Architecture Overview (2 tree) + Layer Rules, Progress Dashboard (openspec kosong → debt di Open Issues), Recent Sessions. RTK section mobile tidak dibawa (tooling khusus mobile). |
| 2026-09-19 | OpenCode | Tema Linen Meridian 2-mode, hapus amoled + gradient | Done | NeedMCP `linen-meridian` token light+dark (Zen Season token kosong). Surfaces/text/border diadopsi, primary tetap coral Kuron. Amoled dihapus (store/CSS/token). Semua gradient → flat solid + hairline (no AI slop). Tracking heading -0.02em. |
| 2026-09-19 | OpenCode | NeedMCP wireframe app-shell (Yoru Sumi) | Done | Style `yoru-sumi` locked (guest, per-call slug). Wireframe `gaming-store-catalog` JSON → `Sidebar` 256 + header 64 + tabs 44 + konten 1180 di `MainPage`. Styling 100% token Kuron (directive: struktur saja). `pnpm check` 0/0, build OK. |
| 2026-09-19 | OpenCode | Liquid Glass coba + revert ke solid | Done | Transparent window + blur/saturate panel ala glass di semua kartu; user: jelek → revert total ke solid Kuron. `.pen` tak terbaca (Pencil MCP transport fail); NeedMCP dikonfirmasi component library (butuh API key), bukan .pen reader. Tema sementara: Solid Kuron. |
| 2026-09-19 | OpenCode | Icon Kuron + window resize + logo fit | Done | `frame.webp` 1536 → `app-icon.png` 1024 → `pnpm tauri icon` regen `src-tauri/icons/` (icns/ico/png). Hapus `tauri.svg` sisa template + favicon. Icon dock macOS: binary embed saat compile → `touch build.rs` + recompile. Splash logo gepeng (900×600 dipaksa kotak) → `height:auto`; ring lingkaran → rounded-rect → dihapus; splash kecil (logo 120) grow 1.4× + window 480×700 → 1200×800 saat ke main (`lib/api/window.ts`, capability window). |
| 2026-09-19 | OpenCode | Assets + clean arch + splash→main + AGENTS/MEMORY | Done | Copy `logo_app.webp`, `frame.webp`, 4 fonts manga dari `learn_flutter/nhasixapp`. Rust: core/domain/application/data/commands/network/cache + `cmd_home_feed` mock. Frontend: route store, `SplashScreen` (logo glow, steps, progress, dots), `MainPage` (featured + grid + backend status), `TagChip`, font-face. `AGENTS.md` + `MEMORY.md` awal. `cargo test` 3 pass. |
| 2026-09-19 | OpenCode | Svelte scaffold + tema Kuron exact | Done | Vite 8 + Svelte 5.57 + TS 5.9 (downgrade dari 7, svelte-check ≤6). `tokens.ts` port `colors_const`/`design_tokens`/`tag_color_palette` 1:1; `app.css` var `--kuron-*` + alias shadcn; switcher dark/light/amoled. `spec KURON_TAURI_V2_DESKTOP.md` → `docs/architecture.md`. |
| 2026-09-19 | OpenCode | Hello world clean-arch init | Done | `core/error` + `domain/entities/hello` + `usecase say_hello` + `cmd_hello_world`/`cmd_app_info`; frontend vanilla `client.js`. Fix `generate_handler!` tolak path → `use` import. `cargo test` 2 pass. |
