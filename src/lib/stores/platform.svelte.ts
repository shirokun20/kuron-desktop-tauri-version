// Platform store — snapshot reaktif (runes) dari `api/platform.ts`, supaya UI
// (shell responsif, overlay, sidebar drawer) ikut berubah tanpa reload.
import {
  detectPlatform,
  subscribePlatform,
  type PlatformSnapshot,
} from "../api/platform";

class PlatformStore {
  snapshot = $state<PlatformSnapshot>(detectPlatform());
  private unsub: (() => void) | null = null;

  get runtime() {
    return this.snapshot.runtime;
  }
  /** Layar sempit → pola mobile. */
  get narrow() {
    return this.snapshot.formFactor === "narrow";
  }

  /** Pasang listener viewport (idempoten; aman dipanggil tiap App mount). */
  init() {
    if (this.unsub) return;
    this.snapshot = detectPlatform();
    this.unsub = subscribePlatform(() => {
      this.snapshot = detectPlatform();
    });
  }
}

export const platformStore = new PlatformStore();
