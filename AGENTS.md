# Kuron Desktop — Agent Rules

**Role**: Senior Tauri v2 + Rust + Svelte Engineer & Architect.
**Goal**: Pure desktop `win+mac+linux` GUI dengan Clean Architecture (spec: `docs/architecture.md`).
**Repo**: `kuron-desktop` — port dari `shirokun20/kuron-mobile` (Flutter Android).

## 📝 Project Memory
**CRITICAL**: Baca `MEMORY.md` di root untuk konteks penuh. Update setelah tiap sesi.

## 🚀 Commands

```bash
pnpm install          # install JS deps
pnpm dev              # tauri dev (Vite :1420 + WebView)
pnpm build            # bundle .dmg/.msi/.AppImage
pnpm check            # svelte-check, harus 0 errors 0 warnings
pnpm build:frontend   # vite build -> dist/ (tanpa WebView)

cd src-tauri && cargo test    # unit tests Rust
cd src-tauri && cargo check   # typecheck cepat Rust
```

macOS desktop-only cukup CLT (`xcode-select --install`); Xcode full ~15GB
**TIDAK wajib** kecuali sign/notarize (spec §19).

## 🏗️ Layer Rules (spec §8–§9)

```
Presentation (Svelte) -> Application (Rust) -> Domain <- Data (Rust)
```

- UI tidak import Data. UI -> `invoke('cmd_*')` -> UseCase -> Repo trait -> DataSource.
- Domain pure: **serde + ts-rs** (ADR-004 generate). No Tauri/SQLite/reqwest.
- Satu use case = satu file, satu method `execute`.
- Commands thin: no logic, map `AppError -> String` di boundary.
- `pnpm check` + `cargo test` hijau sebelum lapor selesai.

## 🎨 Theme

Warna Kuron exact dari kuron-mobile (`DESIGN.md`, `colors_const.dart`).
Token di `src/lib/theme/tokens.ts`, var CSS `--kuron-*` + alias shadcn di
`src/app.css`. Jangan pakai warna default shadcn/Tailwind. Mode: light | dark
(default) via `<html data-theme>`.

## 📁 Struktur

```
src-tauri/src/  core/ domain/ application/ data/ commands/ network.rs cache.rs
src/            main.ts App.svelte app.css assets/
  lib/          api/ domain/ stores/ router/ theme/ components/ pages/
```

Frontend SPA dulu; SvelteKit file-routing Fase 4. Types Rust->TS manual
selama Fase 0 (`ts-rs` generate nyusul ADR-004).

## 📦 Assets

Dari `learn_flutter/nhasixapp/assets/`: `logo_app.webp`, `frame.webp`,
`donation_qris.jpeg` (151KB, dipakai About → Dukung Pengembang),
`legal/id/*.md` (diadaptasi ke desktop: sumber NHentai/Hitomi/E-Hentai/MangaDex,
path Pengaturan mobile dihapus, kontak → repo desktop),
fonts Komika/Bangers/KosugiMaru/ComicNeue di `src/assets/`.
Jangan copy `configs/` (tidak relevan desktop).
App icon: `src/assets/icons/app-icon.png` (1024 PNG dari `frame.webp`) ->
`pnpm tauri icon src/assets/icons/app-icon.png` regenerate `src-tauri/icons/`.
