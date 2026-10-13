// Feature flag AI (deteksi bubble + translate + pilih provider).
// Sumur tunggal — dibaca ReaderPage & SettingsPage.
// Atur via `.env`: VITE_SHOW_AI=true untuk tampilkan, selain itu sembunyi.
// Default sembunyi (permintaan user). Catatan: Vite menanam .env saat
// start/build → setiap ubah .env wajib restart `pnpm dev`.
export const SHOW_AI = import.meta.env.VITE_SHOW_AI === "true";
