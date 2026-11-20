<script>
  // Title bar badges: emulation state and what the game is sending back.
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { ds } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  let st = $derived(ds.status);
</script>

{#if st?.virtualActive}
  <Badge variant="outline" class="border-success/40 text-success">{t("dualsense.badges.emulating")}</Badge>
  {#if st.hidHide === "hiding"}<Badge variant="outline">{t("dualsense.badges.hidden")}</Badge>{/if}
{/if}
