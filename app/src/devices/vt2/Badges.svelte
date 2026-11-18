<script>
  // Title bar badges: live DPI and polling rate.
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { vt2 } from "./store.svelte.js";

  let st = $derived(vt2.status);
  let hz = $derived(st?.polling_hz ? (st.polling_hz >= 1000 ? `${st.polling_hz / 1000} kHz` : `${st.polling_hz} Hz`) : null);
</script>

{#if st}
  <Badge variant="outline" class="font-mono">{st.dpi_x} DPI</Badge>
  {#if hz}<Badge variant="outline" class="font-mono">{hz}</Badge>{/if}
{/if}
