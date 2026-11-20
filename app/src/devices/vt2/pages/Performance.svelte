<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import { vt2, send, POLLING } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

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
  let drain = $derived(t("vt2.performance.drain")[idx]);
</script>

{#if cfg}
  <PageHeader title={t("vt2.performance.title")} description={t("vt2.performance.description")} />

  <div class="grid gap-4">
    <Card.Root>
      <Card.Header>
        <Card.Title>{t("vt2.performance.polling")}</Card.Title>
        <Card.Description>{t("vt2.performance.pollingNote")}</Card.Description>
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
            <div class="text-muted-foreground text-xs">{t("vt2.performance.interval")}</div>
            <div class="mt-1 font-mono text-2xl font-semibold tracking-tight">{interval < 1 ? interval.toFixed(3) : interval.toFixed(1)}<span class="text-muted-foreground ml-1 text-sm">ms</span></div>
          </div>
          <div>
            <div class="text-muted-foreground text-xs">{t("vt2.performance.battery")}</div>
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
          <span class="text-foreground font-medium">{t("vt2.performance.tipLabel")}</span> {t("vt2.performance.tip")}
        </div>
      </Card.Content>
    </Card.Root>
  </div>
{/if}
