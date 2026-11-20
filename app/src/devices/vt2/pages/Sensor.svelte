<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import SettingRow from "$lib/components/setting-row.svelte";
  import { vt2, send } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  let cfg = $derived(vt2.config);

  const toggles = ["motionSync", "linear", "ripple", "glass"];

  function flip(key, v) {
    cfg[key] = v;
    send(key, v);
  }

  let lod = $state(1.1);
  let deb = $state(4);
  $effect(() => {
    if (cfg) {
      lod = cfg.lodMm;
      deb = cfg.debouncePress;
    }
  });
  function commitLod(v) {
    const mm = Math.round(v * 10) / 10;
    if (Math.abs(mm - cfg.lodMm) > 0.01) {
      cfg.lodMm = mm;
      send("lod", mm);
    }
  }
  function commitDeb(v) {
    if (v !== cfg.debouncePress) {
      cfg.debouncePress = v;
      send("debounce", v);
    }
  }
</script>

{#if cfg}
  <PageHeader title={t("vt2.sensor.title")} description={t("vt2.sensor.description")} />

  <div class="grid gap-4">
    <Card.Root class="py-2">
      <Card.Content>
        {#each toggles as key, i}
          {#if i > 0}<Separator />{/if}
          <SettingRow title={t(`vt2.sensor.${key}`)} description={t(`vt2.sensor.${key}Desc`)}>
            <Switch checked={cfg[key]} onCheckedChange={(v) => flip(key, v)} />
          </SettingRow>
        {/each}
      </Card.Content>
    </Card.Root>

    <div class="grid grid-cols-2 gap-4">
      <Card.Root>
        <Card.Header>
          <Card.Description>{t("vt2.sensor.lod")}</Card.Description>
          <Card.Title class="font-mono text-3xl font-semibold tracking-tight">{lod.toFixed(1)}<span class="text-muted-foreground ml-1 text-base">mm</span></Card.Title>
        </Card.Header>
        <Card.Content class="grid gap-3">
          <div class="relative h-16">
            <div class="bg-border absolute inset-x-0 bottom-0 h-px"></div>
            <div
              class="absolute bottom-0 left-1/2 w-24 -translate-x-1/2 transition-transform duration-500 ease-(--ease-out-expo)"
              style="transform: translate(-50%, {-((lod - 0.7) / 1.0) * 36}px)"
            >
              <svg viewBox="0 0 120 44" class="w-full"><path d="M6 40 C 6 18 32 6 64 6 C 96 6 114 22 114 40 Z" class="fill-muted stroke-[oklch(1_0_0/20%)]" stroke-width="1.25" /></svg>
            </div>
          </div>
          <Slider type="single" min={0.7} max={1.7} step={0.1} bind:value={lod} onValueCommit={commitLod} />
          <div class="text-muted-foreground flex justify-between font-mono text-[11px]"><span>0.7</span><span>1.2</span><span>1.7</span></div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Description>{t("vt2.sensor.debounce")}</Card.Description>
          <Card.Title class="font-mono text-3xl font-semibold tracking-tight">{deb}<span class="text-muted-foreground ml-1 text-base">ms</span></Card.Title>
        </Card.Header>
        <Card.Content class="grid gap-3">
          <div class="text-muted-foreground h-16 text-[13px] leading-snug">
            {t("vt2.sensor.debounceDesc")}
          </div>
          <Slider type="single" min={0} max={32} step={1} bind:value={deb} onValueCommit={commitDeb} />
          <div class="text-muted-foreground flex justify-between font-mono text-[11px]"><span>0</span><span>{t("vt2.sensor.onRelease", { ms: cfg.debounceRelease })}</span><span>32</span></div>
        </Card.Content>
      </Card.Root>
    </div>
  </div>
{/if}
