// Kondisi platform — SATU sumber kebenaran untuk keputusan "cara tap" supaya
// desktop (Tauri), web browser, dan mobile (layar sempit) tak punya jalur
// berbeda: popup window native hanya di desktop Tauri layar lebar; sisanya
// overlay in-app (`stores/overlay.svelte`).
//
// MURNI: tanpa import `@tauri-apps/*` → aman di web build + bisa diuji
// `node --test` (`pnpm test`).

export type Runtime = "tauri" | "web";
export type FormFactor = "wide" | "narrow";
/** Tiga konteks yang dipakai UI: desktop Tauri | web browser | mobile. */
export type PlatformMode = "desktop" | "web" | "mobile";

export interface PlatformSnapshot {
  runtime: Runtime;
  formFactor: FormFactor;
}

/** <= ambang ini = pola mobile (sheet full-screen, sidebar jadi drawer). */
export const NARROW_MAX_WIDTH = 900;

interface WinLike {
  innerWidth?: number;
  matchMedia?: (q: string) => { matches: boolean };
  [k: string]: unknown;
}

type ListenerWin = WinLike & {
  addEventListener?: (type: string, cb: () => void) => void;
  removeEventListener?: (type: string, cb: () => void) => void;
};

function resolveWin(w?: WinLike): WinLike | undefined {
  if (w) return w;
  const g = globalThis as { window?: unknown };
  return g.window ? (g.window as WinLike) : undefined;
}

/** True hanya bila halaman jalan di dalam webview Tauri. */
export function isTauriRuntime(w?: WinLike): boolean {
  const x = resolveWin(w);
  if (!x) return false;
  return "__TAURI_INTERNALS__" in x || "__TAURI__" in x;
}

/** Layar sempit (≤ `NARROW_MAX_WIDTH`): pakai pola mobile. */
export function isNarrowViewport(w?: WinLike): boolean {
  const x = resolveWin(w);
  if (!x) return false;
  if (typeof x.matchMedia === "function") {
    try {
      return x.matchMedia(`(max-width: ${NARROW_MAX_WIDTH}px)`).matches;
    } catch {
      // matchMedia menolak query → jatuh ke innerWidth
    }
  }
  return typeof x.innerWidth === "number" ? x.innerWidth <= NARROW_MAX_WIDTH : false;
}

export function detectPlatform(w?: WinLike): PlatformSnapshot {
  return {
    runtime: isTauriRuntime(w) ? "tauri" : "web",
    formFactor: isNarrowViewport(w) ? "narrow" : "wide",
  };
}

/** Label konteks untuk UI/log: desktop | web | mobile. */
export function platformMode(w?: WinLike): PlatformMode {
  const { runtime, formFactor } = detectPlatform(w);
  if (formFactor === "narrow") return "mobile";
  return runtime === "tauri" ? "desktop" : "web";
}

/**
 * Popup window native (Sumber/Filter/Tentang/Ekstensi) hanya masuk akal di
 * Tauri layar lebar. Web & mobile WAJIB lewat overlay in-app.
 */
export function canUseNativeWindows(w?: WinLike): boolean {
  return isTauriRuntime(w) && !isNarrowViewport(w);
}

/** Ikuti perubahan ukuran/orientasi viewport; kembalikan unsubscriber. */
export function subscribePlatform(cb: () => void, w?: WinLike): () => void {
  const x = resolveWin(w) as ListenerWin | undefined;
  const add = x?.addEventListener?.bind(x);
  const remove = x?.removeEventListener?.bind(x);
  if (!add || !remove) return () => {};
  const handler = () => cb();
  add("resize", handler);
  add("orientationchange", handler);
  return () => {
    remove("resize", handler);
    remove("orientationchange", handler);
  };
}
