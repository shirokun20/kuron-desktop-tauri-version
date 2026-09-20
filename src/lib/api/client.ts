import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  Content,
  ExtensionManifest,
  Hello,
  InstalledSource,
} from "../domain/types";

export const DEFAULT_MANIFEST_URL =
  "https://raw.githubusercontent.com/shirokun20/kuron-extensions/main/manifest.json";

export const api = {
  helloWorld: (name?: string): Promise<Hello> =>
    invoke("cmd_hello_world", { name: name ?? null }),
  appInfo: (): Promise<AppInfo> => invoke("cmd_app_info"),
  homeFeed: (source?: string, page?: number): Promise<Content[]> =>
    invoke("cmd_home_feed", { source: source ?? null, page: page ?? null }),
  search: (filter: {
    query: string;
    source_id: string | null;
    page: number;
  }): Promise<Content[]> => invoke("cmd_search", { filter }),
  searchForm: (source: string): Promise<unknown> =>
    invoke("cmd_search_form", { source }),
  tagQuery: (source: string, tagType: string, tagName: string): Promise<string> =>
    invoke("cmd_tag_query", { source, tagType, tagName }),
  sourcesList: (): Promise<InstalledSource[]> => invoke("cmd_sources_list"),
  extManifest: (url?: string): Promise<ExtensionManifest> =>
    invoke("cmd_extension_manifest", { url: url ?? null }),
  extInstall: (manifestUrl: string, id: string): Promise<InstalledSource> =>
    invoke("cmd_extension_install", { manifestUrl, id }),
  extUninstall: (id: string): Promise<boolean> =>
    invoke("cmd_extension_uninstall", { id }),
  extInstallZip: (url: string): Promise<string[]> =>
    invoke("cmd_extension_install_zip_url", { url }),
  extInstallZipFile: (): Promise<string[]> =>
    invoke("cmd_extension_install_zip_file"),
};
