<script>
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { kbd } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";
</script>

{#if kbd.connected}
  <Badge variant="outline" class="font-mono">VIA v{kbd.protocol}</Badge>
  <Badge variant="outline" class="font-mono">{t("via.badges.layers", { n: kbd.layers })}</Badge>
{/if}
