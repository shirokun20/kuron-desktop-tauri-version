// Translation store — state overlay translate per halaman (7.2).
// Backend (cmd_translate_page) yang detect→mosaic→AI→cache; FE hanya
// menyimpan hasil + status + visibilitas overlay. Persist preferensi
// (bahasa target, gaya, skip SFX) di localStorage `kuron.translate`.
import { listen } from "@tauri-apps/api/event";
import { api } from "../api/client";
import { isTauriRuntime } from "../api/platform";
import type { BubbleBox, PageTranslation, TranslationStyle } from "../domain/types";

export type TranslateStage =
  | "idle"
  | "detecting"
  | "translating"
  | "done"
  | "error"
  | "no-provider"
  | "rate-limited";

export const TRANSLATE_KEY = "kuron.translate";

export interface TranslatePrefs {
  targetLang: string;
  style: TranslationStyle;
  skipSfx: boolean;
}

export const DEFAULT_TRANSLATE_PREFS: TranslatePrefs = {
  targetLang: "id",
  style: "natural",
  skipSfx: true,
};

export function loadTranslatePrefs(): TranslatePrefs {
  try {
    const raw = localStorage.getItem(TRANSLATE_KEY);
    if (!raw) return { ...DEFAULT_TRANSLATE_PREFS };
    const o = JSON.parse(raw) as Partial<TranslatePrefs>;
    const styles: TranslationStyle[] = [
      "natural",
      "genz",
      "action",
      "romantis",
      "formal",
      "kasar",
      "literal",
    ];
    return {
      targetLang:
        typeof o.targetLang === "string" && o.targetLang.trim()
          ? o.targetLang.trim()
          : "id",
      style:
        typeof o.style === "string" && styles.includes(o.style as TranslationStyle)
          ? (o.style as TranslationStyle)
          : "natural",
      skipSfx: typeof o.skipSfx === "boolean" ? o.skipSfx : true,
    };
  } catch {
    return { ...DEFAULT_TRANSLATE_PREFS };
  }
}

class TranslateStore {
  /** Hasil per halaman (1-based) — reset tiap ganti bab. */
  results = $state(new Map<number, PageTranslation>());
  /** Bubble terdeteksi per halaman (outline biru, sebelum translate). */
  detected = $state(new Map<number, BubbleBox[]>());
  /** Halaman yang sedang di-detect (anti dobel invoke). */
  detecting = $state(new Set<number>());
  stage = $state<TranslateStage>("idle");
  error = $state<string | null>(null);
  overlayVisible = $state(true);
  prefs = $state<TranslatePrefs>(loadTranslatePrefs());
  private unlisten: (() => void) | null = null;

  patchPrefs(p: Partial<TranslatePrefs>) {
    this.prefs = { ...this.prefs, ...p };
    try {
      localStorage.setItem(TRANSLATE_KEY, JSON.stringify(this.prefs));
    } catch {
      // quota — nilai sesi tetap berlaku
    }
  }

  resultFor(page: number): PageTranslation | undefined {
    return this.results.get(page);
  }

  get busy(): boolean {
    return this.stage === "detecting" || this.stage === "translating";
  }

  againstRateLimit(msg: string): boolean {
    return /rate-limited|429/i.test(msg);
  }

  async startListening() {
    if (this.unlisten || !isTauriRuntime()) return;
    try {
      this.unlisten = await listen<{ stage: string }>("ai:progress", (e) => {
        const s = e.payload?.stage;
        if (s === "detecting") this.stage = "detecting";
        else if (s === "translating") this.stage = "translating";
        else if (s === "done" && this.stage !== "error") this.stage = "done";
      });
    } catch {
      // mode web — tanpa event backend
    }
  }

  stopListening() {
    this.unlisten?.();
    this.unlisten = null;
  }

  resetPage() {
    this.results = new Map();
    this.detected = new Map();
    this.detecting = new Set();
    this.stage = "idle";
    this.error = null;
    this.overlayVisible = true;
  }

  toggleOverlay() {
    this.overlayVisible = !this.overlayVisible;
  }

  clearResult(page: number) {
    this.results.delete(page);
    this.results = new Map(this.results);
  }

  /** Bersihkan bubble terdeteksi halaman ini (outline biru hilang). */
  clearDetected(page: number) {
    this.detected.delete(page);
    this.detected = new Map(this.detected);
  }

  boxesFor(page: number): BubbleBox[] {
    return this.detected.get(page) ?? [];
  }

  /** Deteksi bubble satu halaman (tombol 🛰, manual ala draw-mode).
   *  Murni ONNX lokal — tanpa provider tak apa. Gagal = error tampil di
   *  banner (user yang minta, jadi harus tahu), baca tetap normal. */
  async detectPage(pageUrl: string, sourceId: string, page: number) {
    if (!isTauriRuntime()) return;
    if (this.detected.has(page) || this.detecting.has(page)) return;
    this.detecting.add(page);
    this.detecting = new Set(this.detecting);
    this.error = null;
    try {
      const boxes = await api.detectBubbles(pageUrl, sourceId);
      if (boxes.length > 0) {
        this.detected.set(page, boxes);
        this.detected = new Map(this.detected);
      } else {
        this.error = "Tak ada bubble terdeteksi di halaman ini.";
      }
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.detecting.delete(page);
      this.detecting = new Set(this.detecting);
    }
  }

  /** Ulangi deteksi halaman (abaikan cache) — mis. setelah draw manual. */
  async redetectPage(pageUrl: string, sourceId: string, page: number) {
    this.detected.delete(page);
    this.detected = new Map(this.detected);
    await this.detectPage(pageUrl, sourceId, page);
  }

  /** Terjemahkan satu halaman (backend ambil bytes); hasil per halaman.
   *  Bubble yang sudah terdeteksi (outline biru) dikirim sebagai
   *  `pre_detected` — backend reuse tanpa deteksi ulang. */
  async translatePage(args: {
    pageUrl: string;
    sourceId: string;
    contentId: string;
    pageIndex: number;
    rtl: boolean;
    page: number;
  }) {
    this.stage = "detecting";
    this.error = null;
    try {
      const res = await api.translatePage({
        page_url: args.pageUrl,
        source_id: args.sourceId,
        content_id: args.contentId,
        page_index: args.pageIndex,
        rtl: args.rtl,
        style: this.prefs.style,
        skip_sfx: this.prefs.skipSfx,
        target_lang: this.prefs.targetLang,
        pre_detected: this.boxesFor(args.page),
      });
      this.results.set(args.page, res);
      this.results = new Map(this.results);
      this.stage = "done";
      this.overlayVisible = true;
      return res;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (/belum ada provider|belum punya kunci/i.test(msg)) {
        this.stage = "no-provider";
      } else if (this.againstRateLimit(msg)) {
        this.stage = "rate-limited";
      } else {
        this.stage = "error";
      }
      this.error = msg;
      throw e;
    }
  }
}

export const translateStore = new TranslateStore();
