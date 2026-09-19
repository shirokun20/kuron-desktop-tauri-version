import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri: dev server tetap di 1420 (strictPort), lihat `devUrl` di tauri.conf.json.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
  },
  envPrefix: ["VITE_", "TAURI_"],
});
