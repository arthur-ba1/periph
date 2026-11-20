<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import SettingRow from "$lib/components/setting-row.svelte";
  import OverrideNote from "../override-note.svelte";
  import PageLayout from "../page-layout.svelte";
  import SectionTabs from "../section-tabs.svelte";
  import ColorPopover from "../color-popover.svelte";
  import { ds, send, ui, focusPart, flashPart, partOf, COLORS, PLAYER_PATTERNS, toHex } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  const MODES = ["static", "breathing", "strobe", "rainbow"];
  const SECTIONS = ["lightbar", "player", "mic"];

  let cfg = $derived(ds.config);
  let lb = $derived(cfg?.lightbar);
  let pl = $derived(cfg?.playerLeds);
  let mic = $derived(cfg?.micLed);
  /** Slider values while dragging (sent on release). */
  let drafts = $state({});
  /** The section shown follows the stage (a callout click switches it too). */
  let section = $derived(SECTIONS.includes(ui.focus) ? ui.focus : "lightbar");

  /** Optimistic update of one effect section, then send it whole. */
  function update(key, patch) {
    const value = { ...cfg[key], ...patch };
    cfg[key] = value;
    send(key, value);
    flashPart(partOf(key));
  }

  /** Mode to come back to when the lightbar switch is turned on again. */
  let lastMode = $state("static");
  function setLightbar(on) {
    if (on) update("lightbar", { mode: lastMode });
    else {
      lastMode = lb.mode === "none" ? lastMode : lb.mode;
      update("lightbar", { mode: "none" });
    }
  }

  let preset = $derived(lb ? COLORS.some((c) => toHex(c) === toHex(lb.color)) : true);
  let preview = $derived(lb ? toHex(lb.color.map((c) => Math.round((c * lb.brightness) / 100))) : "#000000");
  let tabItems = $derived(
    cfg
      ? [
          { value: "lightbar", label: t("dualsense.lights.lightbar"), icon: "lightbar", on: lb.mode !== "none" },
          { value: "player", label: t("dualsense.lights.playerShort"), icon: "playerLeds", on: pl.enabled },
          { value: "mic", label: t("dualsense.lights.micShort"), icon: "mic", on: mic.enabled },
        ]
      : [],
  );
</script>

{#if cfg}
  <PageLayout title={t("dualsense.lights.title")} home="lightbar">
    {#snippet tabs()}
      <SectionTabs items={tabItems} value={section} onchange={focusPart} />
    {/snippet}

    {#if section === "lightbar"}
      <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between gap-4">
          <div>
            <Card.Title>{t("dualsense.lights.lightbar")}</Card.Title>
          </div>
          <div class="flex items-center gap-3">
            {#if lb.mode !== "none"}
              <span
                class="size-6 shrink-0 rounded-full border shadow-[0_0_5px_color-mix(in_srgb,var(--glow)_40%,transparent)] transition-[background,box-shadow] {lb.mode === 'rainbow' ? 'rainbow' : ''}"
                style:background={preview}
                style:--glow={preview}
                aria-hidden="true"
              ></span>
            {/if}
            <Switch checked={lb.mode !== "none"} onCheckedChange={setLightbar} aria-label={t("dualsense.lights.lightbar")} />
          </div>
        </Card.Header>
        <Card.Content class="grid gap-4">
          {#if lb.mode !== "none"}
            <OverrideNote checked={lb.override} onchange={(v) => update("lightbar", { override: v })} />
            <ToggleGroup.Root type="single" variant="outline" class="w-full" value={lb.mode} onValueChange={(v) => v && update("lightbar", { mode: v })}>
              {#each MODES as m}
                <ToggleGroup.Item value={m} class="min-w-0 flex-1 px-0.5 text-xs">{t(`dualsense.lights.modes.${m}`)}</ToggleGroup.Item>
              {/each}
            </ToggleGroup.Root>

            {#if lb.mode !== "rainbow"}
              <div class="flex flex-wrap items-center gap-2">
                {#each COLORS as c}
                  {@const hex = toHex(c)}
                  <button
                    type="button"
                    class="size-7 rounded-full transition-transform hover:scale-110 {toHex(lb.color) === hex ? 'ring-foreground ring-2 ring-offset-2 ring-offset-[var(--color-card)]' : ''}"
                    style:background={hex}
                    aria-label={hex}
                    onclick={() => update("lightbar", { color: c })}
                  ></button>
                {/each}
                <ColorPopover value={lb.color} selected={!preset} oninput={(color) => (cfg.lightbar = { ...lb, color })} onchange={(color) => update("lightbar", { color })} />
              </div>
            {/if}

            <div class="grid gap-x-5 gap-y-4 {lb.mode !== 'static' ? 'grid-cols-2' : ''}">
              <div class="grid gap-2">
                <div class="flex justify-between text-sm">
                  <span>{t("dualsense.lights.brightness")}</span>
                  <span class="text-muted-foreground font-mono text-xs">{drafts.brightness ?? lb.brightness}%</span>
                </div>
                <Slider type="single" min={0} max={100} step={5} value={drafts.brightness ?? lb.brightness}
                  onValueChange={(v) => (drafts.brightness = v)} onValueCommit={(v) => { delete drafts.brightness; update("lightbar", { brightness: v }); }} />
              </div>
              {#if lb.mode !== "static"}
                <div class="grid gap-2">
                  <div class="flex justify-between text-sm">
                    <span>{t("dualsense.lights.speed")}</span>
                    <span class="text-muted-foreground font-mono text-xs">{drafts.speed ?? lb.speed}</span>
                  </div>
                  <Slider type="single" min={1} max={10} step={1} value={drafts.speed ?? lb.speed}
                    onValueChange={(v) => (drafts.speed = v)} onValueCommit={(v) => { delete drafts.speed; update("lightbar", { speed: v }); }} />
                </div>
              {/if}
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    {:else if section === "player"}
      <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between gap-4">
          <div>
            <Card.Title>{t("dualsense.lights.player")}</Card.Title>
          </div>
          <Switch checked={pl.enabled} onCheckedChange={(v) => update("playerLeds", { enabled: v })} aria-label={t("dualsense.lights.player")} />
        </Card.Header>
        <Card.Content class="grid gap-4">
          {#if pl.enabled}
            <OverrideNote checked={pl.override} onchange={(v) => update("playerLeds", { override: v })} />
            <!-- same layout as the controller: the middle three close, the outer two set apart -->
            <div class="flex items-center justify-center gap-1.5 rounded-lg bg-black/30 py-4">
              {#each [0, 1, 2, 3, 4] as i}
                {@const on = (pl.mask >> i) & 1}
                <button
                  type="button"
                  class="h-2.5 w-9 rounded-full border transition-[background,box-shadow] {i === 0 ? 'mr-7' : i === 4 ? 'ml-7' : ''} {on ? 'border-white bg-white shadow-[0_0_10px_rgba(255,255,255,0.7)]' : 'bg-muted/40'}"
                  aria-label={t("dualsense.lights.led", { n: i + 1 })}
                  aria-pressed={!!on}
                  onclick={() => update("playerLeds", { mask: pl.mask ^ (1 << i) })}
                ></button>
              {/each}
            </div>
            <div class="grid grid-cols-5 gap-1.5">
              {#each PLAYER_PATTERNS as mask, i}
                <button type="button" class="hover:bg-muted rounded-md border px-1 py-1.5 text-xs transition-colors {pl.mask === mask ? 'border-foreground bg-muted' : ''}" onclick={() => update("playerLeds", { mask })}>
                  {i < 4 ? t("dualsense.lights.playerN", { n: i + 1 }) : t("dualsense.lights.allLeds")}
                </button>
              {/each}
            </div>
            <SettingRow title={t("dualsense.lights.ledBrightness")}>
              <ToggleGroup.Root type="single" variant="outline" value={String(pl.brightness)} onValueChange={(v) => v && update("playerLeds", { brightness: +v })}>
                {#each ["high", "medium", "low"] as level, i}
                  <ToggleGroup.Item value={String(i)} class="px-3 text-xs">{t(`dualsense.lights.levels.${level}`)}</ToggleGroup.Item>
                {/each}
              </ToggleGroup.Root>
            </SettingRow>
          {/if}
        </Card.Content>
      </Card.Root>
    {:else}
      <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between gap-4">
          <div>
            <Card.Title>{t("dualsense.lights.mic")}</Card.Title>
          </div>
          <Switch checked={mic.enabled} onCheckedChange={(v) => update("micLed", { enabled: v })} aria-label={t("dualsense.lights.mic")} />
        </Card.Header>
        <Card.Content class="grid gap-3">
          {#if mic.enabled}
            <OverrideNote checked={mic.override} onchange={(v) => update("micLed", { override: v })} />
            <SettingRow title={t("dualsense.lights.micOn")}>
              <Switch checked={mic.on} onCheckedChange={(v) => update("micLed", { on: v })} />
            </SettingRow>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}
  </PageLayout>
{/if}

<style>
  .rainbow {
    background: conic-gradient(red, yellow, lime, cyan, blue, magenta, red) !important;
  }
</style>
