<script lang="ts">
  // ThemeToggle — switch light/dark. Ikon Sun/Moon referensi Lucide
  // (ISC, https://lucide.dev/icons/sun + /moon), knob geser 200ms.
  import { themeStore } from "../stores/theme.svelte";

  let dark = $derived(themeStore.mode === "dark");

  function toggle() {
    themeStore.set(dark ? "light" : "dark");
  }
</script>

<button
  class="toggle"
  role="switch"
  aria-checked={dark}
  aria-label="Ganti tema gelap terang"
  title={dark ? "Ganti ke terang" : "Ganti ke gelap"}
  onclick={toggle}
>
  <svg class="glyph left" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2" />
    <path d="M12 20v2" />
    <path d="m4.93 4.93 1.41 1.41" />
    <path d="m17.66 17.66 1.41 1.41" />
    <path d="M2 12h2" />
    <path d="M20 12h2" />
    <path d="m6.34 17.66-1.41 1.41" />
    <path d="m19.07 4.93-1.41 1.41" />
  </svg>
  <svg class="glyph right" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401" />
  </svg>
  <span class="knob" class:dark>
    <svg class="glyph sun" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2" />
      <path d="M12 20v2" />
      <path d="m4.93 4.93 1.41 1.41" />
      <path d="m17.66 17.66 1.41 1.41" />
      <path d="M2 12h2" />
      <path d="M20 12h2" />
      <path d="m6.34 17.66-1.41 1.41" />
      <path d="m19.07 4.93-1.41 1.41" />
    </svg>
    <svg class="glyph moon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401" />
    </svg>
  </span>
</button>

<style>
  .toggle {
    position: relative;
    width: 58px;
    height: 30px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--muted);
    cursor: pointer;
    padding: 0;
  }
  .glyph {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    translate: 0 -50%;
    color: var(--muted-foreground);
    pointer-events: none;
  }
  .glyph.left {
    left: 8px;
  }
  .glyph.right {
    right: 8px;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--primary);
    transition: transform 200ms ease;
  }
  .knob.dark {
    transform: translateX(28px);
  }
  .knob .glyph {
    left: 50%;
    translate: -50% -50%;
    width: 14px;
    height: 14px;
    color: var(--primary-foreground);
    transition: opacity 200ms ease, rotate 200ms ease;
  }
  .knob .moon {
    opacity: 0;
    rotate: -90deg;
  }
  .knob.dark .sun {
    opacity: 0;
    rotate: 90deg;
  }
  .knob.dark .moon {
    opacity: 1;
    rotate: 0deg;
  }
</style>
