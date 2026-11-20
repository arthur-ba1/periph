<script>
  // Shown at the top of an effect section whose category the game controls while emulating
  // (chosen in the Emulation page), so it's clear why the setting may not show on the pad.
  import Icon from "$lib/icons/Icon.svelte";
  import { ds } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  /** category: "lights" | "triggers" (keys of config.feedback). */
  let { category } = $props();
  let game = $derived(ds.config?.feedback?.[category] === "game");
</script>

{#if game}
  <div class="bg-muted/30 text-muted-foreground flex items-start gap-2 rounded-lg border px-3 py-2 text-xs leading-relaxed">
    <Icon name="emulate" size={15} class="mt-px shrink-0" />
    <span>{t(`dualsense.sourceNote.${category}`)}</span>
  </div>
{/if}
