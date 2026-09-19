import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, Content, Hello } from "../domain/types";

export const api = {
  helloWorld: (name?: string): Promise<Hello> =>
    invoke("cmd_hello_world", { name: name ?? null }),
  appInfo: (): Promise<AppInfo> => invoke("cmd_app_info"),
  homeFeed: (): Promise<Content[]> => invoke("cmd_home_feed"),
};
