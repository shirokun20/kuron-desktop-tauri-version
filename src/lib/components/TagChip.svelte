<script lang="ts">
  // TagChip — port `TagChip` + `tag_color_palette` kuron-mobile.
  // Warna theme-aware: dark pakai palet dark, light pakai palet light.
  import { tagColor } from "../theme/tokens";
  import { themeStore } from "../stores/theme.svelte";

  interface Props {
    label: string;
    type?: string;
  }

  let { label, type = "tag" }: Props = $props();
  let color = $derived(tagColor(type, themeStore.darkMode));
</script>

<span class="chip" style:--chip={color}>{label}</span>

<style>
  .chip {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    border: 1px solid var(--chip);
    color: var(--chip);
    font-size: 12px;
    font-weight: 500;
  }
</style>
