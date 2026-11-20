<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import AnimatedNumber from "$lib/components/animated-number.svelte";
  import { vt2, send } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  const MIN = 50, MAX = 26000;
  const toPos = (d) => (Math.log(d / MIN) / Math.log(MAX / MIN)) * 1000;
  const toDpi = (p) => Math.min(MAX, Math.max(MIN, Math.round((MIN * Math.pow(MAX / MIN, p / 1000)) / 10) * 10));
  const PRESETS = [400, 800, 1200, 1600, 3200, 6400];

  let cfg = $derived(vt2.config);
  let stages = $derived(cfg ? cfg.dpi.slice(0, cfg.stageCount) : []);
  let editing = $state(null);
  let draft = $state(800);
  let pos = $state(500);

  $effect(() => {
    if (cfg && editing === null) editing = cfg.active;
  });
  $effect(() => {
    if (cfg && editing !== null) setDraft(cfg.dpi[editing]);
  });

  function setDraft(v) {
    draft = v;
    pos = toPos(Math.max(MIN, v || MIN));
  }

  function activate(v) {
    if (v === undefined || v === "") return;
    const i = +v;
    editing = i;
    if (cfg.active !== i) {
      cfg.active = i;
      send("activeStage", i);
    }
  }

  let changed = $derived(cfg && editing !== null && draft !== cfg.dpi[editing]);
  function apply() {
    const v = Math.min(MAX, Math.max(MIN, Math.round(draft / 10) * 10));
    cfg.dpi[editing] = v;
    send("stageDpi", [editing, v]);
  }
  let live = $derived(vt2.status?.dpi_x ?? cfg?.dpi[cfg.active] ?? 0);
</script>

{#if cfg}
  <PageHeader title={t("vt2.dpi.title")} description={t("vt2.dpi.description")} />

  <div class="grid gap-4">
    <Card.Root>
      <Card.Header>
        <Card.Description>{t("vt2.dpi.current")}</Card.Description>
        <Card.Title class="font-mono text-5xl font-semibold tracking-tighter">
          <AnimatedNumber value={live} />
        </Card.Title>
        <Card.Action><Badge variant="outline" class="font-mono">{t("vt2.dpi.stageOf", { n: cfg.active + 1, total: cfg.stageCount })}</Badge></Card.Action>
      </Card.Header>
      <Card.Content>
        <ToggleGroup.Root type="single" variant="outline" class="w-full" value={String(cfg.active)} onValueChange={activate}>
          {#each stages as d, i}
            <ToggleGroup.Item value={String(i)} class="h-14 flex-1 flex-col gap-0.5" aria-label={t("vt2.dpi.stage", { n: i + 1 })}>
              <span class="text-muted-foreground text-[10px] font-medium uppercase tracking-wider">{t("vt2.dpi.stageShort", { n: i + 1 })}</span>
              <span class="font-mono text-sm font-semibold">{d}</span>
            </ToggleGroup.Item>
          {/each}
        </ToggleGroup.Root>
        <div class="mt-3 flex h-6 items-end gap-1.5">
          {#each stages as d, i}
            <div class="bg-muted relative flex-1 overflow-hidden rounded-sm" style="height: 100%">
              <div
                class="absolute inset-x-0 bottom-0 rounded-sm transition-all duration-500 ease-(--ease-out-expo) {cfg.active === i ? 'bg-brand' : 'bg-foreground/15'}"
                style="height:{8 + (toPos(d) / 1000) * 92}%"
              ></div>
            </div>
          {/each}
        </div>
      </Card.Content>
    </Card.Root>

    {#if editing !== null}
      <Card.Root>
        <Card.Header>
          <Card.Title>{t("vt2.dpi.stage", { n: editing + 1 })}</Card.Title>
          <Card.Description>{t("vt2.dpi.range")}</Card.Description>
          <Card.Action>
            <Button onclick={apply} disabled={!changed || vt2.busy}>{t(vt2.busy ? "common.saving" : "common.apply")}</Button>
          </Card.Action>
        </Card.Header>
        <Card.Content class="grid gap-5">
          <div class="flex items-center gap-4">
            <div class="relative w-36">
              <Input
                type="number"
                class="pr-12 font-mono text-base"
                min={MIN}
                max={MAX}
                step={50}
                value={draft}
                oninput={(e) => setDraft(+e.currentTarget.value)}
                onkeydown={(e) => e.key === "Enter" && changed && apply()}
              />
              <span class="text-muted-foreground pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs">DPI</span>
            </div>
            <Slider type="single" min={0} max={1000} step={1} bind:value={pos} onValueChange={(p) => (draft = toDpi(p))} class="flex-1" />
          </div>
          <div class="flex flex-wrap items-center gap-2">
            {#each PRESETS as p}
              <Button size="sm" variant={draft === p ? "secondary" : "outline"} class="font-mono" onclick={() => setDraft(p)}>{p}</Button>
            {/each}
            {#if changed}
              <span class="text-muted-foreground ml-auto text-xs">{t("vt2.dpi.now")} <span class="font-mono">{cfg.dpi[editing]}</span></span>
            {/if}
          </div>
        </Card.Content>
      </Card.Root>
    {/if}
  </div>
{/if}
