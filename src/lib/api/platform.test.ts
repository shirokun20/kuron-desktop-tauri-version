// Kontrak kondisi platform (modul murni, window palsu). `pnpm test`.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  canUseNativeWindows,
  detectPlatform,
  isNarrowViewport,
  isTauriRuntime,
  platformMode,
  subscribePlatform,
  NARROW_MAX_WIDTH,
} from "./platform.ts";

type FakeWin = Record<string, unknown>;

/** window palsu: marker Tauri + lebar + matchMedia (bisa dimatikan). */
function fakeWin(opts: { tauri?: "internals" | "legacy"; width: number; matchMedia?: boolean }): FakeWin {
  const w: FakeWin = { innerWidth: opts.width };
  if (opts.tauri === "internals") w.__TAURI_INTERNALS__ = {};
  if (opts.tauri === "legacy") w.__TAURI__ = {};
  if (opts.matchMedia !== false) {
    w.matchMedia = (q: string) => ({
      matches: Number(q.match(/(\d+)/)?.[1] ?? 0) >= opts.width,
    });
  }
  return w;
}

describe("platform", () => {
  it("Tauri layar lebar = desktop: popup window native boleh", () => {
    const w = fakeWin({ tauri: "internals", width: 1200 });
    assert.deepEqual(detectPlatform(w), { runtime: "tauri", formFactor: "wide" });
    assert.equal(platformMode(w), "desktop");
    assert.equal(canUseNativeWindows(w), true);
  });

  it("Tauri jendela sempit = mobile: native dimatikan, overlay in-app", () => {
    const w = fakeWin({ tauri: "internals", width: 420 });
    assert.equal(platformMode(w), "mobile");
    assert.equal(canUseNativeWindows(w), false);
  });

  it("browser lebar = web: native dimatikan (tak ada window API)", () => {
    const w = fakeWin({ width: 1400 });
    assert.deepEqual(detectPlatform(w), { runtime: "web", formFactor: "wide" });
    assert.equal(platformMode(w), "web");
    assert.equal(canUseNativeWindows(w), false);
  });

  it("browser HP = mobile", () => {
    const w = fakeWin({ width: 390 });
    assert.equal(platformMode(w), "mobile");
    assert.equal(isNarrowViewport(w), true);
    assert.equal(canUseNativeWindows(w), false);
  });

  it("marker legacy __TAURI__ ikut dikenali", () => {
    const w = fakeWin({ tauri: "legacy", width: 1024 });
    assert.equal(isTauriRuntime(w), true);
    assert.equal(platformMode(w), "desktop");
  });

  it("tanpa matchMedia jatuh ke innerWidth; ambang batas inklusif", () => {
    const exact = fakeWin({ tauri: "internals", width: NARROW_MAX_WIDTH, matchMedia: false });
    assert.equal(isNarrowViewport(exact), true);
    const above = fakeWin({ tauri: "internals", width: NARROW_MAX_WIDTH + 1, matchMedia: false });
    assert.equal(isNarrowViewport(above), false);
    assert.equal(canUseNativeWindows(above), true);
  });

  it("tanpa window (node) = web/wide, native mati", () => {
    const w: FakeWin = {};
    assert.deepEqual(detectPlatform(w), { runtime: "web", formFactor: "wide" });
    assert.equal(canUseNativeWindows(w), false);
    assert.equal(subscribePlatform(() => {}, w)(), undefined);
  });

  it("subscribePlatform: resize memicu cb, unsub berhenti", () => {
    const w = fakeWin({ width: 1200 });
    const handlers: Record<string, Array<() => void>> = {};
    w.addEventListener = (t: string, cb: () => void) => (handlers[t] ??= []).push(cb);
    w.removeEventListener = (t: string, cb: () => void) => {
      handlers[t] = (handlers[t] ?? []).filter((h) => h !== cb);
    };
    let hits = 0;
    const unsub = subscribePlatform(() => (hits += 1), w);
    handlers.resize?.forEach((h) => h());
    assert.equal(hits, 1);
    unsub();
    assert.equal(handlers.resize?.length ?? 0, 0);
    assert.equal(handlers.orientationchange?.length ?? 0, 0);
  });
});
