// Window sizing: splash kecil -> main besar (macOS frame).
// No-op di browser (vite preview) — Tauri API tidak ada di sana.
import { getCurrentWindow, LogicalSize, type Window } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const SPLASH = { w: 480, h: 700 };
const MAIN = { w: 1200, h: 800 };

async function current(): Promise<Window | null> {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export async function setSplashSize(): Promise<void> {
  const win = await current();
  if (!win) return;
  try {
    await win.setMinSize(null);
    await win.setSize(new LogicalSize(SPLASH.w, SPLASH.h));
    await win.center();
  } catch {
    // abaikan: capability window belum dibuka / mode browser
  }
}

export async function setMainSize(): Promise<void> {
  const win = await current();
  if (!win) return;
  try {
    await win.setSize(new LogicalSize(MAIN.w, MAIN.h));
    await win.center();
  } catch {
    // abaikan: capability window belum dibuka / mode browser
  }
}

/** Popup window native "Pilih Sumber". Fokuskan bila sudah ada.
 * @returns null bila OK, pesan error bila gagal (ditampilkan di UI). */
export async function openSourcePicker(): Promise<string | null> {
  try {
    let existing = null;
    try {
      existing = await WebviewWindow.getByLabel("source-picker");
    } catch {
      existing = null;
    }
    if (existing) {
      await existing.setFocus().catch(() => {});
      return null;
    }
    const win = new WebviewWindow("source-picker", {
      url: `${window.location.origin}/#source-picker`,
      title: "Pilih Sumber",
      width: 420,
      height: 620,
      center: true,
      resizable: false,
      decorations: true,
      transparent: false,
    });
    return await new Promise<string | null>((resolve) => {
      const timer = setTimeout(
        () =>
          resolve(
            "timeout: window tidak merespons — capability window:create belum aktif? restart pnpm dev",
          ),
        4000,
      );
      win.once("tauri://created", () => {
        clearTimeout(timer);
        resolve(null);
      });
      win.once("tauri://error", (e) => {
        clearTimeout(timer);
        const msg =
          typeof e.payload === "string" ? e.payload : JSON.stringify(e.payload);
        resolve(`gagal buka popup: ${msg}`);
      });
    });
  } catch (e) {
    return `Tauri window API tidak tersedia (mode browser?): ${e}`;
  }
}
