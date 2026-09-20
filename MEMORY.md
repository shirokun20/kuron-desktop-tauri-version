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
| **DI** | `tauri::Builder::manage(AppState)`, ganti GetIt |
| **Networking** | Stub (`network.rs`) — reqwest + cookie_jar + DoH Fase 2 |
| **Routing** | `lib/router/route.svelte.ts` (`splash` \| `main`), SvelteKit Fase 4 |
| **Database** | Stub — `tauri-plugin-sql` SQLite Fase 2 |
| **Logging** | `logger::init()` no-op — `tracing` Fase 0 lengkap |

### Related Repos
| Repo | Role |
|---|---|
| `shirokun20/kuron-mobile` | Ibu: assets, DESIGN.md, pola UI (splash/main/cards) |
| `KURON_TAURI_V2_DESKTOP.md` | Spec arsitektur → `docs/architecture.md` |

### Key Features
- Splash → Main flow (init appInfo + feed saat splash, window resize 480×700 → 1200×800)
- App-shell NeedMCP (sidebar 256 + header 64 + tabs 44 + konten 1180)
- Material rasa Kuron: Light/Dark solid flat-hairline (dial Linen Meridian), tag palette 12 kategori
- Offline-first + AI translate reader (roadmap Fase 2/5, reuse Rust mobile)

---

## Architecture Overview

```
src-tauri/src/
├── core/              # AppError, AppState DI, config, constants, logger(stub)
├── domain/            # Pure Rust — entities, repo traits, value objects, services
│   ├── entities/      # hello, content, chapter, page_image_result, download_task,
│   │                  # ai_translation, glossary, search_filter, reader_settings
│   ├── repositories/  # ContentRepository trait (sync Fase 0, async Fase 1/2)
│   ├── value_objects/ # SourceId, Language
│   └── services/      # title_parser
├── application/       # UseCases — 1 file 1 execute (say_hello, get_home_feed)
├── data/              # MockContentRepository + stub Fase 2/3
│   ├── models/        # ContentModel (From Entity)
│   ├── datasources/   # remote (scraper adapters) + local (sqlite/kv/file) stub
│   ├── repositories/  # mock impl
│   └── native/        # image_ops stub (port kuron_native/rust Fase 3)
├── commands/          # Thin IPC: cmd_hello_world, cmd_app_info, cmd_home_feed
├── network.rs         # HttpClientManager stub (Fase 2)
└── cache.rs           # ImageCache stub (Fase 2)

src/lib/
├── api/               # client.ts (typed invoke) + window.ts (resize)
├── domain/            # types.ts mirror Rust (ts-rs generate nyusul ADR-004)
├── stores/            # hello, content, theme (runes, ganti Cubit/Bloc)
├── router/            # route.svelte.ts, SvelteKit Fase 4
├── theme/             # tokens.ts (port colors/design_tokens/tag palette 1:1)
├── components/        # Sidebar, TagChip, MainGridCard, MainFeaturedCard
└── pages/             # SplashScreen, MainPage
```

### Layer Rules
- **Domain**: Pure Rust + `serde` only. Zero deps ke Data/Presentation/Tauri.
- **Data**: Depends only on Domain. Mock sekarang, remote/local real Fase 2.
- **Application**: Depends only on Domain. Satu use case = satu file + `execute`.
- **Presentation**: Svelte via `invoke('cmd_*')` only. Commands thin, `AppError -> String` di boundary.

---

## Current Progress Dashboard

> Tracked via `openspec/` — Last updated: 2026-09-19

### Archived (in `openspec/changes/archive/`)
- *(none — openspec belum dipakai, semua di Recent Sessions)*

### Active Changes (in `openspec/changes/`)
- *(none)*

### Open Issues (tech debt tercatat, sadar)
- `MockContentRepository` fixture 8 item — ganti real Fase 2
- Repo traits **sync** — jadi async + `Arc<dyn>` bareng SQLite Fase 1/2
- `logger::init()` no-op — `tracing` subscriber Fase 0 lengkap
- `KosugiMaru.ttf` 3.5MB di bundle — subset/lazy-load Fase 8
- SPA + route store manual — SvelteKit Fase 4
- CI matrix 3 OS + signing/updater — Fase 0 lengkap

---

### Recent Sessions

> Session log in this table. Last updated: 2026-09-20.

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
| 2026-09-19 | OpenCode | Tema Linen Meridian 2-mode, hapus amoled + gradient | Done | NeedMCP `linen-meridian` token light+dark (Zen Season token kosong). Surfaces/text/border diadopsi, primary tetap coral Kuron. Amoled dihapus (store/CSS/token). Semua gradient → flat solid + hairline (no AI slop). Tracking heading -0.02em. | Struktur disamakan: Identity + Related Repos + Key Features, Architecture Overview (2 tree) + Layer Rules, Progress Dashboard (openspec kosong → debt di Open Issues), Recent Sessions. RTK section mobile tidak dibawa (tooling khusus mobile). |
| 2026-09-19 | OpenCode | NeedMCP wireframe app-shell (Yoru Sumi) | Done | Style `yoru-sumi` locked (guest, per-call slug). Wireframe `gaming-store-catalog` JSON → `Sidebar` 256 + header 64 + tabs 44 + konten 1180 di `MainPage`. Styling 100% token Kuron (directive: struktur saja). `pnpm check` 0/0, build OK. |
| 2026-09-19 | OpenCode | Liquid Glass coba + revert ke solid | Done | Transparent window + blur/saturate panel ala glass di semua kartu; user: jelek → revert total ke solid Kuron. `.pen` tak terbaca (Pencil MCP transport fail); NeedMCP dikonfirmasi component library (butuh API key), bukan .pen reader. Tema sementara: Solid Kuron. |
| 2026-09-19 | OpenCode | Icon Kuron + window resize + logo fit | Done | `frame.webp` 1536 → `app-icon.png` 1024 → `pnpm tauri icon` regen `src-tauri/icons/` (icns/ico/png). Hapus `tauri.svg` sisa template + favicon. Icon dock macOS: binary embed saat compile → `touch build.rs` + recompile. Splash logo gepeng (900×600 dipaksa kotak) → `height:auto`; ring lingkaran → rounded-rect → dihapus; splash kecil (logo 120) grow 1.4× + window 480×700 → 1200×800 saat ke main (`lib/api/window.ts`, capability window). |
| 2026-09-19 | OpenCode | Assets + clean arch + splash→main + AGENTS/MEMORY | Done | Copy `logo_app.webp`, `frame.webp`, 4 fonts manga dari `learn_flutter/nhasixapp`. Rust: core/domain/application/data/commands/network/cache + `cmd_home_feed` mock. Frontend: route store, `SplashScreen` (logo glow, steps, progress, dots), `MainPage` (featured + grid + backend status), `TagChip`, font-face. `AGENTS.md` + `MEMORY.md` awal. `cargo test` 3 pass. |
| 2026-09-19 | OpenCode | Svelte scaffold + tema Kuron exact | Done | Vite 8 + Svelte 5.57 + TS 5.9 (downgrade dari 7, svelte-check ≤6). `tokens.ts` port `colors_const`/`design_tokens`/`tag_color_palette` 1:1; `app.css` var `--kuron-*` + alias shadcn; switcher dark/light/amoled. `spec KURON_TAURI_V2_DESKTOP.md` → `docs/architecture.md`. |
| 2026-09-19 | OpenCode | Hello world clean-arch init | Done | `core/error` + `domain/entities/hello` + `usecase say_hello` + `cmd_hello_world`/`cmd_app_info`; frontend vanilla `client.js`. Fix `generate_handler!` tolak path → `use` import. `cargo test` 2 pass. |
