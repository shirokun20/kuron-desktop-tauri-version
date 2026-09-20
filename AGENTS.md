# Kuron Desktop — Agent & Developer Rules

## Identitas proyek

- **Peran:** Senior Tauri v2, Rust, dan Svelte engineer.
- **Target:** pure desktop GUI Windows, macOS, dan Linux.
- **Stack:** Tauri v2 + Rust + Svelte 5 + TypeScript + Vite.
- **Acuan parity:** [`shirokun20/kuron-mobile`](https://github.com/shirokun20/kuron-mobile).
- **Arsitektur:** Clean Architecture; detail ada di [`docs/architecture.md`](docs/architecture.md).

Kuron Desktop bukan wrapper mobile. UI harus terasa desktop, tetapi perilaku
source, search, filter, dan tema dipertahankan sedekat mungkin dengan Kuron Mobile.

## Startup wajib

Sebelum membaca atau mengubah kode:

1. Baca `MEMORY.md`.
2. Baca `openspec/changes/` dan cari change yang masih aktif.
3. Jika ada change aktif, baca `proposal.md`, `design.md`, dan `tasks.md`.
4. Pastikan pekerjaan yang dilakukan memang tercakup dalam task atau bug yang
   diminta user.
5. Periksa `git status` dan jangan menghapus perubahan user.

Update `MEMORY.md` setelah sesi yang menghasilkan perubahan atau keputusan
teknis penting.

## Perintah utama

```bash
pnpm install
pnpm dev                 # Tauri dev + Vite
pnpm dev:frontend        # Vite saja, tanpa backend Rust
pnpm check               # svelte-check; target 0 error, 0 warning
pnpm test                # Node tests frontend
pnpm build:frontend      # Vite production build
pnpm build               # bundle installer Tauri

cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo clean --manifest-path src-tauri/Cargo.toml
```

`cargo clean` hanya menghapus artefak `src-tauri/target/`; gunakan saat cache
build Rust membesar. Jangan menghapus source, resource, database user, atau
folder data aplikasi untuk sekadar membersihkan project.

## Workflow OpenSpec

Gunakan workflow OpenSpec untuk perubahan fitur atau perubahan arsitektur besar:

- **Explore:** pahami masalah dan codebase; jangan mengubah kode.
- **Propose:** buat proposal, design, spec delta, dan tasks.
- **Apply:** implementasikan task satu per satu dan tandai task selesai.
- **Sync:** sinkronkan delta ke main specs bila diminta.
- **Archive:** arsipkan change hanya setelah implementasi dan validasi selesai.

Jangan membuat planning Markdown baru di root. Gunakan artifact OpenSpec atau
file session yang sesuai. Untuk bug kecil, tetap baca konteks yang relevan dan
buat perubahan sekecil mungkin.

## Aturan layer

```text
Presentation (Svelte) -> Application (Rust) -> Domain <- Data (Rust)
```

- **Presentation:** komponen, halaman, stores, router, dan typed API bridge.
- **Application:** use case yang mengorkestrasi alur aplikasi.
- **Domain:** entity, value object, service, dan repository trait yang murni.
- **Data:** datasource, HTTP, SQLite, cache, config, dan adapter konkret.
- **Commands:** boundary IPC yang tipis; validasi boundary dan pemetaan
  `AppError -> String` boleh dilakukan di sini.

Aturan keras:

- UI tidak mengimpor Data Rust.
- UI memakai `invoke('cmd_*')` melalui wrapper di `src/lib/api/`.
- Domain tidak boleh bergantung pada Tauri, SQLite, reqwest, atau WebView.
- Satu use case = satu file dan satu method `execute`.
- Jangan menaruh business logic di command Tauri.
- Gunakan repository trait untuk membalik dependency Data ke Domain.
- Pertahankan type safety; hindari `as any` dan `as unknown as`.

## Svelte dan tema

- Gunakan Svelte 5 runes (`$state`, `$derived`, `$effect`) sesuai pola repo.
- Event handler mengikuti pola Svelte yang sudah dipakai project.
- Jalankan `pnpm check` setelah perubahan frontend.
- Gunakan token di `src/lib/theme/tokens.ts` dan alias Kuron di `src/app.css`.
- Jangan menambahkan warna default Tailwind/shadcn jika token Kuron tersedia.
- Mode tema yang didukung: `light` dan `dark`; pertahankan atribut
  `<html data-theme>`.
- Browser mode harus tetap aman tanpa backend Tauri; jangan memanggil invoke
  secara diam-diam dari browser.
- Overlay in-app dipakai untuk browser/mobile viewport; native window hanya
  jika runtime Tauri dan alur memang membutuhkannya.

## Config-driven source system

Source search/filter harus berasal dari config, bukan hardcode per situs.
Perhatikan:

- `src-tauri/resources/source-configs/`
- `src-tauri/src/data/datasources/config.rs`
- `src-tauri/src/data/datasources/extension.rs`
- `src/lib/pages/FilterPage.svelte`
- `src/lib/api/client.ts`

Saat mengubah source atau extension:

- Pertahankan validasi path dan source ID.
- ZIP harus dipreview dan di-install secara selective; jangan memasang semua
  config tanpa persetujuan user.
- Jangan mengubah URL manifest/ZIP menjadi default otomatis di form.
- Meta ikon boleh berasal dari manifest atau asset ZIP; path lokal harus
  diubah menjadi URL yang dapat dibaca WebView.
- `nhentai` bawaan tidak boleh di-uninstall lewat UI.

## Rust dan generated types

- Rust format dengan `cargo fmt`; jangan melakukan format manual yang luas.
- Domain memakai `serde` dan `ts-rs` sesuai pola yang ada.
- Jika tipe Rust yang diekspor berubah, regen `src/lib/domain/types.ts` lewat
  mekanisme repo; jangan mengedit generated output tanpa alasan kuat.
- Error harus eksplisit. Jangan memakai broad catch, silent fallback, atau
  return sukses ketika operasi sebenarnya gagal.
- Log tidak boleh berisi credential, token, prompt, atau data sensitif.

## Testing dan definition of done

Sebelum melaporkan pekerjaan selesai:

1. Jalankan test paling kecil yang mencakup perubahan.
2. Jalankan `pnpm check` untuk perubahan frontend.
3. Jalankan `cargo test --manifest-path src-tauri/Cargo.toml --lib` untuk
   perubahan backend atau kontrak IPC.
4. Jalankan `git diff --check`.
5. Untuk perubahan UI penting, verifikasi pada aplikasi berjalan atau browser
   dev server, bukan hanya lewat pembacaan kode.
6. Catat command dan hasil penting di respons serta update `MEMORY.md`.

Jangan mengklaim test hijau jika test gagal karena sandbox, network, atau
dependency. Laporkan jumlah pass/fail/ignored dan penyebabnya.

## Asset dan perubahan file

- Gunakan asset yang sudah ada di `src/assets/` atau resource yang sesuai.
- Jangan menyalin folder `configs/` mobile ke bundle desktop; source tambahan
  dipasang melalui extension system.
- Jangan mengubah file yang tidak berkaitan.
- Jangan menjalankan `git reset --hard`, `git checkout --`, atau perintah
  destruktif lain tanpa persetujuan user.
- Jangan commit secret, credential, atau file data pribadi.
- Jangan membuat commit kecuali diminta user.

## Referensi penting

- [`README.md`](README.md) — quick start dan orientasi proyek.
- [`MEMORY.md`](MEMORY.md) — konteks lintas sesi.
- [`docs/architecture.md`](docs/architecture.md) — arsitektur lengkap.
- [`openspec/changes/`](openspec/changes/) — perubahan aktif.
- [`shirokun20/kuron-mobile`](https://github.com/shirokun20/kuron-mobile) — acuan
  perilaku, desain, dan parity mobile.
- [`shirokun20/kuron-extensions`](https://github.com/shirokun20/kuron-extensions) —
  repo extension komunitas.
