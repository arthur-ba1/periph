<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import { dev, send, POLLING } from "../store.svelte.js";

  let cfg = $derived(dev.config);
  const fmt = (hz) => (hz >= 1000 ? `${hz / 1000} kHz` : `${hz} Hz`);
</script>

{#if cfg}
  <PageHeader title="Desempenho" description="Taxa de atualização do dispositivo." />

  <Card.Root>
    <Card.Header>
      <Card.Title>Polling rate</Card.Title>
      <Card.Description>Quantas vezes por segundo o dispositivo informa sua posição.</Card.Description>
    </Card.Header>
    <Card.Content>
      <ToggleGroup.Root type="single" variant="outline" value={String(cfg.polling)} onValueChange={(v) => v && send("polling", +v)}>
        {#each POLLING as hz}
          <ToggleGroup.Item value={String(hz)} class="font-mono">{fmt(hz)}</ToggleGroup.Item>
        {/each}
      </ToggleGroup.Root>
    </Card.Content>
  </Card.Root>
{/if}
