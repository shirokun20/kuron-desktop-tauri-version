// Kuron theme tokens — port 1:1 dari kuron-mobile:
//   lib/core/constants/colors_const.dart (AppColors)
//   lib/core/constants/design_tokens.dart (spacing/radius/duration)
//   lib/core/utils/tag_color_palette.dart (12 kategori tag)
// Sumber: https://github.com/shirokun20/kuron-mobile/blob/master/DESIGN.md

export type KuronThemeMode = "light" | "dark";

export interface KuronTheme {
  bg: string;
  surface: string;
  card: string;
  cardAlt: string;
  border: string;
  text: string;
  textSub: string;
  readerBg: string;
  readerText: string;
}

export const brand = {
  coral: "#F1958E",
  muted: "#E0827E",
  dusty: "#9D555B",
  dark: "#1A1A1F",
  warm: "#D48A6A",
  warmMuted: "#B87054",
  readGold: "#C8A06A",
  readGoldLight: "#B89060",
} as const;

export const status = {
  error: "#C86858",
  success: "#8AB87A",
  warning: "#D4A060",
  info: "#7BB8FF",
} as const;

export const themes: Record<KuronThemeMode, KuronTheme> = {
  // default Kuron = dark
  dark: {
    bg: "#1A1714",
    surface: "#292420",
    card: "#332C26",
    cardAlt: "#221E1A",
    border: "#3E362E",
    text: "#F5EFE6",
    textSub: "#A89C8C",
    readerBg: "#1A1614",
    readerText: "#D0C8C0",
  },
  light: {
    bg: "#FAF7F2",
    surface: "#F5F0E8",
    card: "#FFFFFF",
    cardAlt: "#F0EAE0",
    border: "#E7DED1",
    text: "#1C1B1A",
    textSub: "#8A7E6E",
    readerBg: "#F5EDE4",
    readerText: "#2E2722",
  },
};

// primary light butuh coral lebih gelap demi kontras AA (lightCoral #C76A62)
export const primaryFor = (mode: KuronThemeMode): string =>
  mode === "light" ? "#C76A62" : brand.coral;

// --- spacing / radius / duration (design_tokens.dart) ---
export const space = {
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 24,
  xxl: 32,
  xxxl: 48,
} as const;

export const radius = {
  sm: 4,
  md: 8,
  lg: 12,
  xl: 16,
  xxl: 20,
  full: 999,
} as const;

export const duration = {
  instant: 50,
  fast: 150,
  pageTurn: 200,
  normal: 300,
  slow: 500,
  pageEnter: 700,
} as const;

// --- tag palette (tag_color_palette.dart), light & dark ---
const tagLight: Record<string, string> = {
  artist: "#B8655E",
  character: "#4F7FB8",
  parody: "#AE6D67",
  group: "#A85A5A",
  language: "#B8873B",
  category: "#7D7269",
  uploader: "#5FA383",
  female: "#B85F87",
  male: "#5A86C2",
  other: "#8A6B5C",
  misc: "#8B7C57",
  tag: "#BF6C63",
};

const tagDark: Record<string, string> = {
  artist: brand.coral,
  character: brand.muted,
  parody: brand.dusty,
  group: "#B77A74",
  language: "#C2A46E",
  category: "#8A847E",
  uploader: "#6F9F87",
  female: "#C98A9B",
  male: "#7FA5C9",
  other: "#7E726D",
  misc: "#9B8F67",
  tag: "#B8776D",
};

const fallbackLight = ["#BF6C63", "#5A86C2", "#AE6D67", "#B55A5A", "#B8873B", "#5FA383", "#8A6B5C", "#7D7269"];
const fallbackDark = [brand.coral, brand.muted, brand.dusty, "#C2A46E", "#6F9F87", "#7FA5C9", "#9B8F67", "#B8776D"];

function stableHash(value: string): number {
  let hash = 0;
  for (const ch of value) {
    const code = ch.codePointAt(0) ?? 0;
    hash = 0x1fffffff & (hash + code);
    hash = 0x1fffffff & (hash + ((hash & 0x0007ffff) << 10));
    hash ^= hash >> 6;
  }
  hash = 0x1fffffff & (hash + ((hash & 0x03ffffff) << 3));
  hash ^= hash >> 11;
  hash = 0x1fffffff & (hash + ((hash & 0x00003fff) << 15));
  return hash;
}

/** Tag -> warna, theme-aware. `darkMode` true untuk mode dark. */
export function tagColor(tagType: string, darkMode: boolean): string {
  const fixed = darkMode ? tagDark : tagLight;
  const fallback = darkMode ? fallbackDark : fallbackLight;
  const key = tagType.trim().toLowerCase();
  if (!key) return fallback[0];
  return fixed[key] ?? fallback[stableHash(key) % fallback.length];
}
