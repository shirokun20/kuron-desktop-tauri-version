import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "./platform";
import type {
  AppInfo,
  Content,
  ExtensionManifest,
  Hello,
  InstalledSource,
} from "../domain/types";

export const DEFAULT_MANIFEST_URL =
  "https://raw.githubusercontent.com/shirokun20/kuron-extensions/main/manifest.json";

/** Mode web (dibuka via browser): backend Rust tidak ada. */
export const WEB_OFFLINE_MSG =
  "mode web: backend Rust tidak aktif — jalankan `pnpm dev` untuk data nyata";

/** Command Rust; di web ditolak dengan pesan jelas (bukan error kriptik). */
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) return Promise.reject(new Error(WEB_OFFLINE_MSG));
  return invoke<T>(cmd, args);
}

export const api = {
  // Identitas aplikasi tetap dijawab di web supaya splash/Sidebar/About jalan
  // (tidak ada data sumber yang bisa dilayani tanpa backend).
  helloWorld: (name?: string): Promise<Hello> =>
    isTauriRuntime()
      ? invoke("cmd_hello_world", { name: name ?? null })
      : Promise.resolve({
          name: name ?? "web",
          message: "Mode web: UI jalan tanpa backend Rust.",
        }),
  appInfo: (): Promise<AppInfo> =>
    isTauriRuntime()
      ? invoke("cmd_app_info")
      : Promise.resolve({
          name: "Kuron Desktop",
          version: "0.1.0-web",
          backend: "web (tanpa backend Rust)",
        }),
  homeFeed: (source?: string, page?: number): Promise<Content[]> =>
    call("cmd_home_feed", { source: source ?? null, page: page ?? null }),
  search: (filter: {
    query: string;
    source_id: string | null;
    page: number;
  }): Promise<Content[]> => call("cmd_search", { filter }),
  searchForm: (source: string): Promise<unknown> =>
    call("cmd_search_form", { source }),
  tagQuery: (source: string, tagType: string, tagName: string): Promise<string> =>
    call("cmd_tag_query", { source, tagType, tagName }),
  // Web: daftar kosong (Sidebar tetap menampilkan sumber tersimpan user).
  sourcesList: (): Promise<InstalledSource[]> =>
    isTauriRuntime() ? invoke("cmd_sources_list") : Promise.resolve([]),
  extManifest: (url?: string): Promise<ExtensionManifest> =>
    call("cmd_extension_manifest", { url: url ?? null }),
  extInstall: (manifestUrl: string, id: string): Promise<InstalledSource> =>
    call("cmd_extension_install", { manifestUrl, id }),
  extUninstall: (id: string): Promise<boolean> =>
    call("cmd_extension_uninstall", { id }),
  extInstallZip: (url: string): Promise<string[]> =>
    call("cmd_extension_install_zip_url", { url }),
  extInstallZipFile: (): Promise<string[]> =>
    call("cmd_extension_install_zip_file"),
};
