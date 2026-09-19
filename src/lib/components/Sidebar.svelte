<script lang="ts">
  // Sidebar — port `app_main_drawer_widget.dart`: header profil (logo ring
  // coral + pill subtitle), source selector (popover sheet ala bottomSheet),
  // grup BERANDA/EXPLORE/MORE, item chevron + indikator aktif, footer versi.
  import appIcon from "../../assets/icons/app-icon.png";
  import { openSourcePicker } from "../api/window";

  export interface NavItem {
    id: string;
    label: string;
    icon: string;
  }

  export interface NavGroup {
    label: string;
    items: NavItem[];
  }

  interface Props {
    groups: NavGroup[];
    active: string;
    collapsed: boolean;
    source: string;
    version: string;
    onSelect: (id: string) => void;
  }

  let { groups, active, collapsed, source, version, onSelect }: Props = $props();
  let pickerError = $state<string | null>(null);

  async function openPicker() {
    pickerError = null;
    pickerError = await openSourcePicker();
  }

  const CHEV_RIGHT = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>';
  const CHEV_EXPAND = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>';
</script>

<aside class="sidebar" class:collapsed>
  {#if !collapsed}
    <div class="profile">
      <img class="avatar" src={appIcon} alt="Kuron" />
      <strong class="appname">Kuron</strong>
      <span class="tagline">Klien tidak resmi Nhentai</span>
    </div>

    <button class="source" onclick={openPicker} title="Ganti sumber (popup window)">
      <span class="tile s">{source.slice(0, 1).toUpperCase()}</span>
      <span class="source-name">{source}</span>
      <span class="chev">{@html CHEV_EXPAND}</span>
    </button>
    {#if pickerError}
      <p class="picker-err">{pickerError}</p>
    {/if}
  {/if}

  <nav class="side-nav">
    {#each groups as group (group.label)}
      {#if !collapsed}
        <p class="group-label">{group.label}</p>
      {/if}
      {#each group.items as item (item.id)}
        <button
          class:active={active === item.id}
          onclick={() => onSelect(item.id)}
          title={item.label}
        >
          <span class="tile">{@html item.icon}</span>
          {#if !collapsed}
            <span class="lbl">{item.label}</span>
            <span class="chev">{@html CHEV_RIGHT}</span>
          {/if}
        </button>
      {/each}
    {/each}
  </nav>

  {#if !collapsed}
    <div class="side-cta">
      <span class="tile k">K</span>
      <span class="ver"><strong>Kuron</strong><small>v{version}</small></span>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    width: 256px;
    flex-shrink: 0;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--popover);
    border-right: 1px solid var(--border);
    transition: width 200ms ease;
    overflow: hidden;
  }
  .sidebar.collapsed {
    width: 64px;
  }
  .profile {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px 16px 16px;
    text-align: center;
  }
  .avatar {
    width: 88px;
    height: 88px;
    border-radius: 50%;
    object-fit: cover;
    border: 2px solid var(--primary);
  }
  .appname {
    margin-top: 12px;
    font-size: 24px;
    color: var(--primary);
    letter-spacing: -0.02em;
  }
  .tagline {
    margin-top: 6px;
    font-size: 12px;
    color: var(--muted-foreground);
    background: var(--muted);
    border-radius: 999px;
    padding: 4px 12px;
  }
  .source {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 4px 16px 8px;
    padding: 8px 12px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: var(--foreground);
    font-size: 15px;
    cursor: pointer;
    text-align: left;
  }
  .source:hover {
    background: var(--muted);
  }
  .source-name {
    flex: 1;
  }
  .picker-err {
    margin: 0 16px 8px;
    font-size: 12px;
    color: var(--destructive);
  }
  .side-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    overflow-y: auto;
  }
  .group-label {
    margin: 14px 12px 6px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--primary);
    opacity: 0.8;
  }
  .side-nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: var(--muted-foreground);
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
    text-align: left;
  }
  .collapsed .side-nav button {
    justify-content: center;
    padding: 9px 0;
  }
  .side-nav button:hover {
    background: var(--muted);
    color: var(--foreground);
  }
  .side-nav button.active {
    background: color-mix(in srgb, var(--primary) 14%, transparent);
    color: var(--primary);
    font-weight: 600;
  }
  .side-nav button.active::before {
    content: "";
    position: absolute;
    left: -12px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 3px;
    background: var(--primary);
  }
  .tile {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: var(--muted);
    font-weight: 700;
  }
  .tile :global(svg) {
    width: 18px;
    height: 18px;
  }
  button.active .tile {
    background: color-mix(in srgb, var(--primary) 22%, transparent);
    color: var(--primary);
  }
  .tile.s, .tile.k {
    color: var(--primary);
  }
  .lbl {
    flex: 1;
  }
  .chev {
    display: inline-flex;
    color: var(--muted-foreground);
    opacity: 0.6;
  }
  .chev :global(svg) {
    width: 16px;
    height: 16px;
  }
  .side-cta {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 8px 16px 16px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--card);
  }
  .ver {
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .ver strong {
    font-size: 14px;
  }
  .ver small {
    font-size: 12px;
    color: var(--muted-foreground);
  }
</style>
