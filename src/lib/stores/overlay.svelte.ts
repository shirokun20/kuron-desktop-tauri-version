// Overlay store — SATU pintu "tap" untuk Sumber/Tentang/Ekstensi/Filter, agar
// cara tap sama di semua konteks:
//   desktop Tauri layar lebar → popup window native (seperti sekarang)
//   web browser & mobile      → overlay in-app (sheet / full-screen)
// Semua tombol UI memanggil `overlayStore.open(kind)`; keputusan ada di sini.
import { canUseNativeWindows } from "../api/platform";
import { openWindowPopup, type WindowPopupKind } from "../api/window";

export type OverlayKind = WindowPopupKind;

class OverlayStore {
  /** Jenis overlay in-app yang sedang tampil (`null` = tidak ada). */
  kind = $state<OverlayKind | null>(null);
  /** Pesan bila popup window native gagal dibuka (desktop). */
  error = $state<string | null>(null);

  get inApp(): boolean {
    return this.kind !== null;
  }

  async open(kind: OverlayKind): Promise<void> {
    this.error = null;
    if (canUseNativeWindows()) {
      this.kind = null;
      this.error = await openWindowPopup(kind);
      return;
    }
    // Tap jenis yang sama = tutup (toggle) — satu perilaku di web & mobile.
    this.kind = this.kind === kind ? null : kind;
  }

  close() {
    this.kind = null;
    this.error = null;
  }
}

export const overlayStore = new OverlayStore();
