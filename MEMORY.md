# Kuron Desktop — Project Memory

> Konteks lintas sesi & AI tools. Baca `AGENTS.md` untuk aturan kerja.
> Port dari `shirokun20/kuron-mobile` (Flutter Android `0.9.26+36`).

---

## Project Identity

| Key | Value |
|---|---|
| **App** | Kuron Desktop `0.1.0` (`id.nhasix.kuron`) |
| **Platform** | Pure desktop win+mac+linux GUI (Tauri v2) |
| **Stack** | Rust (Tokio) + TypeScript + Vite 8 + Svelte 5.57 |
| **Arch** | Clean: Domain <- Data, Application -> Domain, Presentation via IPC |
| **State** | Svelte 5 runes stores (`*.svelte.ts`), ganti Cubit |
| **DI** | `tauri::Builder::manage(AppState)` (minimal), ganti GetIt |
| **Routing** | `lib/router/route.svelte.ts` (`splash` \| `main`), SvelteKit Fase 4 |
| **Theme** | Warna Kuron exact, 3 mode (dark default / light / amoled) |
| **Ibu** | `learn_flutter/nhasixapp` (sumber assets + DESIGN.md + pola UI) |

## Architecture Map

```
src-tauri/src/
  core/         error.rs AppError | di.rs AppState | config | constants | logger(stub)
  domain/       entities/{hello,content,chapter,page_image_result,download_task,
                ai_translation,glossary,search_filter,reader_settings}
                repositories/content_repository.rs (trait sync Fase 0)
                value_objects/{source_id,language} | services/title_parser
  application/  usecases/{say_hello,get_home_feed}
  data/         models/content_model | repositories MockContentRepository
                datasources/{remote,local} stub | native/image_ops stub
  commands/     hello_commands (cmd_hello_world, cmd_app_info)
                content_commands (cmd_home_feed mock)
  network.rs cache.rs stub | lib.rs manage(AppState) + 4 commands
src/lib/
  api/client.ts | domain/types.ts (manual, ts-rs nyusul)
  stores/{hello,content,theme}.svelte.ts | router/route.svelte.ts
  theme/tokens.ts | components/{TagChip,MainGridCard,MainFeaturedCard}
  pages/{SplashScreen,MainPage}
src/assets/     icons/{logo_app,frame}.webp | fonts/{Komika,Bangers,KosugiMaru,ComicNeue}
```

## Progress

- [x] Fase 0a: hello world IPC (`cmd_hello_world`, `cmd_app_info`)
- [x] Frontend Svelte + tema Kuron + tag palette + fonts manga
- [x] Assets dari nhasixapp (logo, frame, 4 fonts)
- [x] Struktur clean-arch Rust penuh (mock repo, stub Fase 2/3)
- [x] Splash -> Main flow (SplashScreen init appInfo+feed, route store)
- [x] AGENTS.md + MEMORY.md
- [ ] Fase 0 lengkap: tracing, AppState DI Arc<dyn>, CI matrix 3 OS
- [ ] Fase 1: entities penuh + ts-rs generate types.ts
- [ ] Fase 2: scraper adapters, SQLite, reqwest+DoH, image cache
- [ ] Fase 3: image_ops + ort bubble_detector port
- [ ] Fase 4: SvelteKit + Home/Search/Detail real
- [ ] Fase 5: Reader canvas + TranslationOverlay polygon + draw mode

## Tech Debt (sadar, tercatat)

1. `MockContentRepository` fixture 8 item — ganti real Fase 2.
2. Repo traits **sync** — jadi async + `Arc<dyn>` bareng SQLite Fase 1/2.
3. `logger::init()` no-op — `tracing` subscriber Fase 0 lengkap.
4. `KosugiMaru.ttf` 3.5MB di bundle — pertimbangkan subset/lazy-load Fase 8.
5. SPA + route store manual — SvelteKit Fase 4.

## Conventions

- Rust: `snake_case` file/fn, `cmd_` prefix commands, `AppError -> String` di boundary.
- Svelte: runes (`$state/$derived`), stores class di `*.svelte.ts`, style scoped per komponen.
- Warna: selalu dari `tokens.ts` / var CSS, never hardcode hex di komponen.
- Card: flat + 1px border (`DESIGN.md`: elevation 0), radius 16.
