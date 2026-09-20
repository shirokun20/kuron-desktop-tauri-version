# Kuron Desktop

<p align="center">
  <strong>Pure desktop manga reader berbasis Tauri v2, Rust, dan Svelte.</strong>
</p>

<p align="center">
  <a href="https://tauri.app/">Tauri v2</a> ·
  <a href="https://www.rust-lang.org/">Rust</a> ·
  <a href="https://svelte.dev/">Svelte 5</a> ·
  <a href="https://vite.dev/">Vite</a> ·
  Windows · macOS · Linux
</p>

> Kuron Desktop adalah port desktop dari [Kuron Mobile](https://github.com/shirokun20/kuron-mobile).
> Fokusnya adalah aplikasi desktop native yang ringan, offline-first, dan tetap
> memakai source system config-driven milik Kuron.

## Status

| Area | Status |
|---|---|
| Platform | Windows, macOS, Linux |
| UI | Svelte 5 + TypeScript + Vite |
| Backend | Rust + Tauri v2 |
| Arsitektur | Clean Architecture |
| Source bawaan | NHentai |
| Source tambahan | Manifest URL atau import ZIP |
| Routing | SPA; migrasi SvelteKit direncanakan |

Proyek ini masih aktif dikembangkan. Sebagian fitur desktop lanjutan seperti
download manager, reader penuh, dan AI translation mengikuti roadmap porting
dari Kuron Mobile.

## Fitur utama

- **Desktop native** — satu codebase untuk Windows, macOS, dan Linux.
- **Clean Architecture** — Presentation → Application → Domain ← Data.
- **Config-driven sources** — source baru tidak membutuhkan perubahan kode inti.
- **Source manager** — pasang source dari manifest URL atau ZIP.
- **Selective ZIP install** — pilih source mana yang ingin dipasang sebelum instalasi.
- **Source icons** — ikon dari manifest atau asset lokal di dalam ZIP.
- **Search dan filter dinamis** — form mengikuti konfigurasi source, termasuk filter
  tag MangaDex dengan mode include/exclude.
- **Tema Kuron** — light dan dark menggunakan token warna Kuron, bukan warna default
  Tailwind/shadcn.
- **Offline-first foundation** — cache, SQLite, key-value storage, dan keychain
  disiapkan di backend Rust.
- **Responsive presentation** — overlay in-app untuk browser/mobile viewport dan
  native window untuk desktop Tauri jika diperlukan.

## Screenshot

Screenshot desktop akan ditambahkan setelah layout dan alur utama stabil. Untuk
referensi desain, lihat [DESIGN.md di Kuron Mobile](https://github.com/shirokun20/kuron-mobile/blob/main/DESIGN.md).

## Tech stack

| Layer | Teknologi |
|---|---|
| Frontend | Svelte 5, TypeScript, Vite |
| Desktop shell | Tauri v2 |
| Backend | Rust, Tokio, reqwest |
| Storage | SQLite, file cache, KV JSON, keychain |
| Serialization | Serde, `ts-rs` |
| Architecture | Clean Architecture + thin IPC commands |
| Testing | `svelte-check`, Node tests, Rust unit tests |

## Struktur project

```text
src/                         # Presentation Svelte
├── App.svelte
├── app.css
└── lib/
    ├── api/                 # typed IPC/browser bridge
    ├── components/          # komponen UI
    ├── domain/              # tipe frontend
    ├── pages/               # halaman dan overlay
    ├── router/              # route SPA
    ├── stores/              # state Svelte 5 runes
    └── theme/               # token tema Kuron

src-tauri/src/               # Backend Rust
├── core/                    # AppState, error, DI, logger
├── domain/                  # entity, value object, repository trait
├── application/             # use case; satu file, satu execute
├── data/                    # config, datasource, adapter, repository
├── commands/                # thin Tauri IPC boundary
├── network.rs
└── cache.rs

src-tauri/resources/         # config source bawaan dan resource runtime
docs/                        # arsitektur dan dokumentasi teknis
openspec/                    # proposal, spec, design, dan tasks perubahan
```

## Quick start

### Prasyarat

- Node.js dan pnpm
- Rust toolchain (`rustup`)
- Prasyarat Tauri sesuai OS:
  [tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/)

Untuk development macOS, Command Line Tools cukup. Xcode penuh hanya diperlukan
untuk kebutuhan tertentu seperti signing atau notarization.

### Jalankan development

```bash
pnpm install
pnpm dev
```

Frontend-only:

```bash
pnpm dev:frontend
```

### Validasi

```bash
pnpm check
pnpm test
pnpm build:frontend
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
```

### Build installer

```bash
pnpm build
```

Output installer mengikuti target OS Tauri, misalnya `.dmg`, `.msi`, atau
`.AppImage`.

## Source dan extension

Source tambahan mengikuti format config Kuron dan dikelola lewat halaman
**Ekstensi**:

1. Isi URL `manifest.json` sendiri.
2. Klik **Muat** untuk melihat daftar source.
3. Pasang source satu per satu, atau pilih file/URL ZIP.
4. Untuk ZIP, centang source yang diinginkan lalu klik **Pasang terpilih**.
5. Source yang sudah terpasang dapat dihapus dari bagian **Terpasang**.

URL manifest dan ZIP sengaja kosong saat awal untuk menghindari download
otomatis tanpa persetujuan pengguna. Repo extension komunitas:
[`shirokun20/kuron-extensions`](https://github.com/shirokun20/kuron-extensions).

## Membersihkan artefak build

Jika cache Rust membuat project membesar, hapus artefak build dengan:

```bash
cargo clean --manifest-path src-tauri/Cargo.toml
```

Perintah ini tidak menghapus source code atau konfigurasi. Folder
`src-tauri/target/` akan dibuat kembali saat test atau build berikutnya.

## Arsitektur dan aturan kontribusi

- [docs/architecture.md](docs/architecture.md) — keputusan arsitektur dan layer.
- [AGENTS.md](AGENTS.md) — aturan kerja agent dan developer.
- [MEMORY.md](MEMORY.md) — konteks lintas sesi.
- [openspec/changes/](openspec/changes/) — perubahan aktif dan task implementasi.

Sebelum mengubah kode, baca `AGENTS.md`, `MEMORY.md`, dan OpenSpec change yang
aktif. Perubahan UI tidak boleh mengimpor layer Data Rust secara langsung;
gunakan bridge IPC atau fallback browser yang sudah tersedia.

## Legal

Kuron adalah proyek komunitas untuk tujuan edukasi dan penggunaan pribadi.
Source pihak ketiga dapat memiliki ketentuan penggunaan dan hak cipta masing-masing.
Hormati pemilik konten dan gunakan source sesuai hukum yang berlaku.

Lisensi proyek mengikuti [LICENSE](LICENSE) jika file tersebut tersedia di
distribusi ini.
