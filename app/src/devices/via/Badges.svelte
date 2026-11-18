<script>
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { kbd } from "./store.svelte.js";
</script>

{#if kbd.connected}
  <Badge variant="outline" class="font-mono">VIA v{kbd.protocol}</Badge>
  <Badge variant="outline" class="font-mono">{kbd.layers} camadas</Badge>
{/if}
