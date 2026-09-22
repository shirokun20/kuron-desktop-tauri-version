// Pengaturan store — snapshot reaktif (runes) dari `settingsPersist`;
// tiap `patch` langsung persist ke localStorage (9.1 / spec settings-ai).
import {
  loadSettings,
  saveSettings,
  type AppSettings,
} from "./settingsPersist";

class SettingsStore {
  s = $state<AppSettings>(loadSettings());

  get blurThumbnail(): boolean {
    return this.s.blurThumbnail;
  }
  get autoLoadFeed(): boolean {
    return this.s.autoLoadFeed;
  }

  /** Tulis sebagian field — hasil ternormalisasi mengganti seluruh state. */
  patch(p: Partial<AppSettings>) {
    this.s = saveSettings(p);
  }
}

export const settingsStore = new SettingsStore();
