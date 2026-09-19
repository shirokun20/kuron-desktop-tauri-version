# Kuron Desktop — Tauri v2 + Svelte (Fase 0: Hello World)

Pure desktop `win+mac+linux` GUI. Spec penuh: `docs/architecture.md`.

## Frontend: Vite + Svelte 5 + TypeScript

```
src/
  main.ts / App.svelte (splash -> main shell) / app.css
  assets/icons/{logo_app,frame}.webp  assets/fonts/{Komika,Bangers,KosugiMaru,ComicNeue}
  lib/
    api/client.ts                  # typed invoke wrappers (spec §15)
    domain/types.ts                # TS mirror Rust entities (ts-rs nyusul)
    stores/{hello,content,theme}.svelte.ts  # runes ganti Cubit/Bloc
    router/route.svelte.ts         # 'splash'|'main', SvelteKit Fase 4
    theme/tokens.ts                # palette Kuron exact (brand/tag/spacing)
    components/{TagChip,MainGridCard,MainFeaturedCard}
    pages/{SplashScreen,MainPage}
```

Backend Rust clean (spec §9): `core/` (error, AppState DI, config,
constants, logger) | `domain/` (9 entities + repo traits + value objects +
title_parser) | `application/` (usecases) | `data/` (ContentModel,
MockContentRepository, datasource/native stubs Fase 2/3) | `commands/`
(`cmd_hello_world`, `cmd_app_info`, `cmd_home_feed` mock).

SvelteKit file-routing nyusul Fase 4; SPA ini cukup untuk IPC proof.

## Tema: warna Kuron, struktur shadcn

Port 1:1 dari `kuron-mobile` (`colors_const.dart`, `design_tokens.dart`,
`tag_color_palette.dart`, `DESIGN.md`):

- brand: coral `#F1958E`, muted `#E0827E`, dusty `#9D555B`, dark `#1A1A1F`
- 3 mode: **dark** (default) `#1C1816`, **light** `#EFE6DC`, **amoled** `#000000`
- Var `--kuron-*` nilai exact; alias shadcn (`--background`, `--primary`,
  `--card`, …) map ke token Kuron → `shadcn-svelte` drop-in nanti.

## Backend: Rust clean (spec §9)

```
src-tauri/src/
  core/         # AppError
  domain/       # entities/hello.rs — pure, zero deps
  application/  # usecases/say_hello.rs — 1 file 1 execute
  commands/     # cmd_hello_world, cmd_app_info — thin IPC
```

## Jalanin

```bash
pnpm install
pnpm dev       # tauri dev (Vite :1420 + WebView)
pnpm build     # bundle .dmg/.msi/.AppImage

pnpm check            # svelte-check (0 errors)
pnpm build:frontend   # vite build -> dist/
cd src-tauri && cargo test
```
