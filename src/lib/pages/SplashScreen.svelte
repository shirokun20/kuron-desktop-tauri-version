<script lang="ts">
  // SplashScreen — port `splash_screen.dart`: logo glow coral,
  // status init berurutan, progress bar, dots, lalu -> main.
  import { onMount } from "svelte";
  import logo from "../../assets/icons/logo_app.webp";
  import { contentStore } from "../stores/content.svelte";
  import { helloStore } from "../stores/hello.svelte";
  import { routeStore } from "../router/route.svelte";

  const STEPS = [
    "Menyiapkan komponen…",
    "Memuat konfigurasi…",
    "Menghubungkan…",
    "Siap!",
  ];

  let step = $state(0);
  let progress = $derived(Math.min(1, (step + 1) / STEPS.length));
  let timers: ReturnType<typeof setTimeout>[] = [];

  onMount(() => {
    // init beneran: app info + feed dimuat saat splash (ala SplashBloc)
    helloStore.loadInfo();
    contentStore.load();

    STEPS.forEach((_, i) => {
      timers.push(setTimeout(() => (step = i), 350 * (i + 1)));
    });
    timers.push(setTimeout(() => routeStore.go("main"), 350 * STEPS.length + 400));
    return () => timers.forEach(clearTimeout);
  });
</script>

<div class="splash">
  <div class="logo-ring">
    <img src={logo} alt="Kuron" width="160" height="160" />
  </div>
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
  .logo-ring {
    padding: 20px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--kuron-primary) 30%, transparent);
    box-shadow: 0 0 30px 4px color-mix(in srgb, var(--kuron-primary) 20%, transparent);
    background: color-mix(in srgb, var(--popover) 30%, transparent);
  }
  .logo-ring img {
    display: block;
    border-radius: 24px;
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
