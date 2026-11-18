<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import { vt2, send, POLLING } from "../store.svelte.js";

  let cfg = $derived(vt2.config);
  let idx = $derived(cfg ? Math.max(0, POLLING.indexOf(cfg.polling)) : 0);
  const label = (hz) => (hz >= 1000 ? `${hz / 1000}K` : `${hz}`);

  function set(v) {
    const hz = +v;
    if (!v || cfg.polling === hz) return;
    cfg.polling = hz;
    send("polling", hz);
  }

  let interval = $derived(cfg ? 1000 / cfg.polling : 0);
  let drain = $derived(["Mínimo", "Muito baixo", "Baixo", "Moderado", "Alto", "Muito alto", "Máximo"][idx]);
</script>

{#if cfg}
  <PageHeader title="Desempenho" description="Quantas vezes por segundo o mouse envia a posição ao PC." />

  <div class="grid gap-4">
    <Card.Root>
      <Card.Header>
        <Card.Title>Polling rate</Card.Title>
        <Card.Description>Taxas acima de 1 kHz exigem o dongle de alta velocidade.</Card.Description>
      </Card.Header>
      <Card.Content class="grid gap-6">
        <ToggleGroup.Root type="single" variant="outline" class="w-full" value={String(cfg.polling)} onValueChange={set}>
          {#each POLLING as hz}
            <ToggleGroup.Item value={String(hz)} class="h-12 flex-1 flex-col gap-0">
              <span class="font-mono text-sm font-semibold">{label(hz)}</span>
              <span class="text-muted-foreground text-[10px]">Hz</span>
            </ToggleGroup.Item>
          {/each}
        </ToggleGroup.Root>

        <div class="grid grid-cols-2 gap-6">
          <div>
            <div class="text-muted-foreground text-xs">Intervalo entre reports</div>
            <div class="mt-1 font-mono text-2xl font-semibold tracking-tight">{interval < 1 ? interval.toFixed(3) : interval.toFixed(1)}<span class="text-muted-foreground ml-1 text-sm">ms</span></div>
          </div>
          <div>
            <div class="text-muted-foreground text-xs">Consumo de bateria</div>
            <div class="mt-1 text-2xl font-semibold tracking-tight">{drain}</div>
            <Progress value={((idx + 1) / POLLING.length) * 100} class="mt-2 h-1" />
          </div>
        </div>
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Content class="flex gap-3">
        <Icon name="info" class="text-muted-foreground mt-0.5" />
        <div class="text-muted-foreground text-sm leading-relaxed">
          <span class="text-foreground font-medium">Recomendação:</span> 1–2 kHz para uso diário. 4–8 kHz reduzem a latência em poucos décimos de milissegundo, mas consomem muito mais bateria e CPU.
        </div>
      </Card.Content>
    </Card.Root>
  </div>
{/if}
