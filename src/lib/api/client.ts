import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "./platform";
import type {
  AiModelOption,
  AiProvider,
  AiProviderInput,
  AiProviderKind,
  AppInfo,
  Chapter,
  Comment,
  Content,
  ExtensionManifest,
  Hello,
  HistoryItem,
  InstalledSource,
  PageImageResult,
  ZipPreview,
} from "../domain/types";

/** Mode web (dibuka via browser): backend Rust tidak ada. */
export const WEB_OFFLINE_MSG =
  "mode web: backend Rust tidak aktif — data nyata hanya di aplikasi desktop";

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
  detail: (contentId: string, source?: string): Promise<Content> =>
    call("cmd_get_detail", { contentId, source: source ?? null }),
  chapters: (
    contentId: string,
    source?: string,
    language?: string | null,
    offset?: number | null,
  ): Promise<Chapter[]> =>
    call("cmd_get_chapters", {
      contentId,
      source: source ?? null,
      language: language ?? null,
      offset: offset ?? null,
    }),
  pageImages: (chapterId: string, source?: string): Promise<PageImageResult[]> =>
    call("cmd_get_page_images", { chapterId, source: source ?? null }),
  // image_ops (Fase 3): payload base64; backend spawn_blocking.
  imageChunkWebtoon: (data: string, maxChunkH: number): Promise<string[]> =>
    call("cmd_image_chunk_webtoon", { data, maxChunkH }),
  imageBuildMosaic: (data: string, boxes: number[]): Promise<string> =>
    call("cmd_image_build_mosaic", { data, boxes }),
  imageCompressPage: (data: string, maxDim: number): Promise<string> =>
    call("cmd_image_compress_page", { data, maxDim }),
  related: (contentId: string, source?: string): Promise<Content[]> =>
    call("cmd_get_related", { contentId, source: source ?? null }),
  comments: (contentId: string, source?: string): Promise<Comment[]> =>
    call("cmd_get_comments", { contentId, source: source ?? null }),
  // Web: daftar kosong (Sidebar tetap menampilkan sumber tersimpan user).
  sourcesList: (): Promise<InstalledSource[]> =>
    isTauriRuntime() ? invoke("cmd_sources_list") : Promise.resolve([]),
  extManifest: (url?: string): Promise<ExtensionManifest> =>
    call("cmd_extension_manifest", { url: url ?? null }),
  extInstall: (manifestUrl: string, id: string): Promise<InstalledSource> =>
    call("cmd_extension_install", { manifestUrl, id }),
  extUninstall: (id: string): Promise<boolean> =>
    call("cmd_extension_uninstall", { id }),
  extPreviewZip: (url: string): Promise<ZipPreview> =>
    call("cmd_extension_preview_zip_url", { url }),
  extPreviewZipFile: (): Promise<ZipPreview> =>
    call("cmd_extension_install_zip_file"),
  extInstallStagedZip: (token: string, selected: string[]): Promise<string[]> =>
    call("cmd_extension_install_staged_zip", { token, selected }),
  historyRecord: (content: Content, position: number): Promise<void> =>
    call("cmd_history_record", { content, position }),
  historyList: (limit?: number): Promise<HistoryItem[]> =>
    call("cmd_history_list", { limit: limit ?? null }),
  historyRemove: (contentId: string): Promise<void> =>
    call("cmd_history_remove", { contentId }),
  historyClear: (): Promise<void> => call("cmd_history_clear"),
  favoriteSet: (content: Content, fav: boolean): Promise<void> =>
    call("cmd_favorite_set", { content, fav }),
  favoriteList: (): Promise<Content[]> => call("cmd_favorite_list"),
  libraryClear: (): Promise<void> => call("cmd_library_clear"),
  // AI BYOK (9.2): kunci hanya lewat argumen save; list tak pernah balikkan kunci.
  aiProvidersList: (): Promise<AiProvider[]> => call("cmd_ai_providers_list"),
  aiProviderSave: (provider: AiProviderInput, apiKey: string): Promise<AiProvider> =>
    call("cmd_ai_provider_save", { provider, apiKey }),
  aiProviderDelete: (id: string): Promise<boolean> =>
    call("cmd_ai_provider_delete", { id }),
  // LOV model live (GET /models per jenis); apiKey = yang diketik form,
  // dipakai sekali di backend untuk listing, tak disimpan.
  aiModelCatalog: (kind: AiProviderKind, apiKey: string): Promise<AiModelOption[]> =>
    call("cmd_ai_model_catalog", { kind, apiKey }),
};
