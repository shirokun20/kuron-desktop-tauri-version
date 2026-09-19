// Theme store — ganti `theme_cubit` (spec §16).
// Mode persist di localStorage, default `dark` (default Kuron).
import type { KuronThemeMode } from "../theme/tokens";

const KEY = "kuron-theme";
const MODES: KuronThemeMode[] = ["light", "dark"];

function initial(): KuronThemeMode {
  const saved = localStorage.getItem(KEY);
  return MODES.includes(saved as KuronThemeMode)
    ? (saved as KuronThemeMode)
    : "dark";
}

class ThemeStore {
  mode = $state<KuronThemeMode>(initial());
  darkMode = $derived(this.mode === "dark");

  constructor() {
    this.apply(this.mode);
  }

  set(mode: KuronThemeMode) {
    this.mode = mode;
    this.apply(mode);
  }

  private apply(mode: KuronThemeMode) {
    document.documentElement.dataset.theme = mode;
    localStorage.setItem(KEY, mode);
  }
}

export const themeStore = new ThemeStore();
export { MODES };
