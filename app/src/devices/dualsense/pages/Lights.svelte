<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import SettingRow from "$lib/components/setting-row.svelte";
  import SourceNote from "../source-note.svelte";
  import PageLayout from "../page-layout.svelte";
  import SectionTabs from "../section-tabs.svelte";
  import ColorPopover from "../color-popover.svelte";
  import { ds, send, ui, focusPart, flashPart, partOf, COLORS, PLAYER_PATTERNS, mirrorBit, toHex } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  const MODES = ["static", "breathing", "strobe", "rainbow"];
  // player and mic LEDs share one section ("player"); a click on the mic callout opens it too
  const SECTIONS = ["lightbar", "player"];

  let cfg = $derived(ds.config);
  let lb = $derived(cfg?.lightbar);
  let pl = $derived(cfg?.playerLeds);
  /** Periph lights the lightbar ("none": never set, "off": kept dark). */
  let lit = $derived(!!lb && lb.mode !== "none" && lb.mode !== "off");
  let mic = $derived(cfg?.micLed);
  /** Slider values while dragging (sent on release). */
  let drafts = $state({});
  /** The section shown follows the stage (a callout click switches it too). */
  let section = $derived(ui.focus === "mic" ? "player" : SECTIONS.includes(ui.focus) ? ui.focus : "lightbar");

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
      lastMode = lit ? lb.mode : lastMode;
      // off = dark, not "leave it alone" (the pad would keep its last colour)
      update("lightbar", { mode: "off" });
    }
  }

  let preset = $derived(lb ? COLORS.some((c) => toHex(c) === toHex(lb.color)) : true);
  let preview = $derived(lb ? toHex(lb.color.map((c) => Math.round((c * lb.brightness) / 100))) : "#000000");
  let tabItems = $derived(
    cfg
      ? [
          { value: "lightbar", label: t("dualsense.lights.lightbar"), icon: "lightbar", on: lit },
          { value: "player", label: t("dualsense.lights.playerShort"), icon: "playerLeds", on: pl.enabled || mic.mode === "on" || mic.mode === "blink" },
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
            {#if lit}
              <span
                class="size-6 shrink-0 rounded-full border shadow-[0_0_5px_color-mix(in_srgb,var(--glow)_40%,transparent)] transition-[background,box-shadow] {lb.mode === 'rainbow' ? 'rainbow' : ''}"
                style:background={preview}
                style:--glow={preview}
                aria-hidden="true"
              ></span>
            {/if}
            <Switch checked={lit} onCheckedChange={setLightbar} aria-label={t("dualsense.lights.lightbar")} />
          </div>
        </Card.Header>
        <Card.Content class="grid gap-4">
          {#if lit}
            <SourceNote category="lights" />
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
    {:else}
      <Card.Root>
        <Card.Header>
          <Card.Title>{t("dualsense.lights.leds")}</Card.Title>
        </Card.Header>
        <Card.Content class="grid gap-4">
          <SourceNote category="lights" />
          <SettingRow title={t("dualsense.lights.player")}>
            <Switch checked={pl.enabled} onCheckedChange={(v) => update("playerLeds", { enabled: v })} aria-label={t("dualsense.lights.player")} />
          </SettingRow>
          {#if pl.enabled}
            <!-- same layout as the controller: the middle three close, the outer two set apart.
                 The pad mirrors them around the centre, so a click lights the LED and its twin. -->
            <div class="flex items-center justify-center gap-1.5 rounded-lg bg-black/30 py-4">
              {#each [0, 1, 2, 3, 4] as i}
                {@const on = (pl.mask >> i) & 1}
                <button
                  type="button"
                  class="h-2.5 w-9 rounded-full border transition-[background,box-shadow] {i === 0 ? 'mr-7' : i === 4 ? 'ml-7' : ''} {on ? 'border-white bg-white shadow-[0_0_10px_rgba(255,255,255,0.7)]' : 'bg-muted/40'}"
                  aria-label={t("dualsense.lights.led", { n: i + 1 })}
                  aria-pressed={!!on}
                  onclick={() => update("playerLeds", { mask: on ? pl.mask & ~mirrorBit(i) : pl.mask | mirrorBit(i) })}
                ></button>
              {/each}
            </div>
            <!-- player 1–4 on one line, then none / all as their own group -->
            {#snippet pattern(mask, label)}
              <button type="button" class="hover:bg-muted min-w-0 truncate rounded-md border px-1 py-1.5 text-xs transition-colors {pl.mask === mask ? 'border-foreground bg-muted' : ''}" onclick={() => update("playerLeds", { mask })}>
                {label}
              </button>
            {/snippet}
            <div class="flex items-center gap-3">
              <div class="grid flex-[4] grid-cols-4 gap-1.5">
                {#each PLAYER_PATTERNS.slice(1, 5) as mask, i}{@render pattern(mask, t("dualsense.lights.playerN", { n: i + 1 }))}{/each}
              </div>
              <div class="bg-border h-5 w-px shrink-0"></div>
              <div class="grid flex-[2] grid-cols-2 gap-1.5">
                {@render pattern(PLAYER_PATTERNS[0], t("dualsense.lights.noLeds"))}
                {@render pattern(PLAYER_PATTERNS[5], t("dualsense.lights.allLeds"))}
              </div>
            </div>
            <SettingRow title={t("dualsense.lights.ledBrightness")}>
              <ToggleGroup.Root type="single" variant="outline" value={String(pl.brightness)} onValueChange={(v) => v && update("playerLeds", { brightness: +v })}>
                {#each ["high", "medium", "low"] as level, i}
                  <ToggleGroup.Item value={String(i)} class="px-3 text-xs">{t(`dualsense.lights.levels.${level}`)}</ToggleGroup.Item>
                {/each}
              </ToggleGroup.Root>
            </SettingRow>
          {/if}
          <Separator />
          <!-- "auto" (never set: Periph leaves the LED alone) reads as off, the pad's own state -->
          <SettingRow title={t("dualsense.lights.mic")}>
            <ToggleGroup.Root type="single" variant="outline" value={mic.mode === "auto" ? "off" : mic.mode} onValueChange={(v) => v && update("micLed", { mode: v })}>
              {#each ["off", "on", "blink"] as m}
                <ToggleGroup.Item value={m} class="px-3 text-xs">{t(`dualsense.lights.micModes.${m}`)}</ToggleGroup.Item>
              {/each}
            </ToggleGroup.Root>
          </SettingRow>
          {#if mic.mode === "blink"}
            <div class="grid gap-2">
              <div class="flex justify-between text-sm">
                <span>{t("dualsense.lights.micInterval")}</span>
                <span class="text-muted-foreground font-mono text-xs">{t("dualsense.lights.ms", { ms: drafts.micInterval ?? mic.intervalMs })}</span>
              </div>
              <Slider type="single" min={100} max={2000} step={50} value={drafts.micInterval ?? mic.intervalMs}
                onValueChange={(v) => (drafts.micInterval = v)} onValueCommit={(v) => { delete drafts.micInterval; update("micLed", { intervalMs: v }); }} />
            </div>
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
