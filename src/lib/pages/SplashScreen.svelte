<script lang="ts">
  // SplashScreen — port `splash_screen.dart`: logo glow coral,
  // status init berurutan, progress bar, dots, lalu -> main.
  import { onMount } from "svelte";
  import logo from "../../assets/icons/logo_app.webp";
  import { contentStore } from "../stores/content.svelte";
  import { helloStore } from "../stores/hello.svelte";
  import { routeStore } from "../router/route.svelte";
  import { setMainSize, setSplashSize } from "../api/window";

  const STEPS = [
    "Menyiapkan komponen…",
    "Memuat konfigurasi…",
    "Menghubungkan…",
    "Siap!",
  ];

  let step = $state(0);
  let growing = $state(false);
  let progress = $derived(Math.min(1, (step + 1) / STEPS.length));
  let timers: ReturnType<typeof setTimeout>[] = [];

  onMount(() => {
    // init beneran: app info + feed dimuat saat splash (ala SplashBloc)
    helloStore.loadInfo();
    contentStore.load();
    setSplashSize();

    STEPS.forEach((_, i) => {
      timers.push(setTimeout(() => (step = i), 350 * (i + 1)));
    });
    // logo membesar + window melebar sesaat sebelum pindah main
    timers.push(setTimeout(() => (growing = true), 350 * STEPS.length));
    timers.push(
      setTimeout(async () => {
        await setMainSize();
        routeStore.go("main");
      }, 350 * STEPS.length + 450),
    );
    return () => timers.forEach(clearTimeout);
  });
</script>

<div class="splash">
  <img class="logo" class:grow={growing} src={logo} alt="Kuron" />
  <h1>Kuron</h1>
  <p class="subtitle">Desktop reader — privacy-first, offline-ready</p>

  <div class="status">{STEPS[step]}</div>
  <div class="bar">
    <div class="fill" style:width={`${progress * 100}%`}></div>
  </div>
  <div class="pct">{Math.round(progress * 100)}%</div>
  <div class="dots">
    {#each [0, 1, 2] as i}
      <span class:active={step % 3 === i}></span>
    {/each}
  </div>
  {#if helloStore.info}
    <p class="ver">{helloStore.info.name} v{helloStore.info.version}</p>
  {/if}
</div>

<style>
  .splash {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: var(--kuron-reader-bg);
    text-align: center;
    padding: 24px;
  }
  .logo {
    display: block;
    width: 120px;
    height: auto;
    transition: transform 450ms ease;
  }
  .logo.grow {
    transform: scale(1.4);
  }
  h1 {
    margin: 32px 0 4px;
    letter-spacing: 1.2px;
  }
  .subtitle {
    color: var(--muted-foreground);
    font-style: italic;
    margin: 0 0 32px;
  }
  .status {
    background: color-mix(in srgb, var(--popover) 50%, transparent);
    border: 1px solid var(--border);
    border-radius: 20px;
    padding: 8px 24px;
    font-weight: 600;
  }
  .bar {
    width: min(320px, 70vw);
    height: 6px;
    border-radius: 4px;
    background: var(--muted);
    margin-top: 20px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--primary);
    transition: width 300ms ease;
  }
  .pct {
    color: var(--primary);
    font-weight: 700;
    font-size: 14px;
    margin-top: 8px;
  }
  .dots {
    display: flex;
    gap: 8px;
    margin-top: 16px;
  }
  .dots span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted-foreground);
    opacity: 0.4;
  }
  .dots span.active {
    background: var(--primary);
    opacity: 1;
  }
  .ver {
    color: var(--muted-foreground);
    font-size: 12px;
    margin-top: 24px;
  }
</style>
