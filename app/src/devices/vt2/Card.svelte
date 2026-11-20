<script>
  // Sidebar card: render thumbnail, connection, battery. Right click picks the mouse colour
  // the renders show (like the DualSense card).
  import { fade } from "svelte/transition";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import BatteryGlyph, { batteryState, BATTERY_BAR } from "$lib/icons/Battery.svelte";
  import { ContextMenu } from "bits-ui";
  import CheckIcon from "@lucide/svelte/icons/check";
  import { COLORS } from "./components/mouse-render.svelte";
  import { vt2, prefs, setColor } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  const thumbs = import.meta.glob("./assets/thumb_*.png", { eager: true, import: "default" });

  let st = $derived(vt2.status);
  let battery = $derived(st?.battery ?? null);
  let batState = $derived(batteryState(battery, !!st?.charging));
  let asleep = $derived(!vt2.connected && vt2.code === "asleep");
  let thumb = $derived(thumbs[`./assets/thumb_${prefs.color}.png`]);
</script>

<ContextMenu.Root>
<ContextMenu.Trigger class="border-sidebar-border bg-card block rounded-lg border p-3">
  <div class="relative mb-3 flex h-28 items-center justify-center overflow-hidden rounded-md bg-gradient-to-b from-white/[0.04] to-transparent">
    {#if thumb}
      {#key prefs.color}
        <img src={thumb} alt="" class="h-24 drop-shadow-[0_8px_16px_rgba(0,0,0,0.6)] transition-opacity {vt2.connected ? '' : 'opacity-40 grayscale'}" in:fade={{ duration: 300 }} draggable="false" />
      {/key}
    {:else}
      <Icon name="mouse" size={40} class="text-muted-foreground" />
    {/if}
  </div>
  <div class="flex items-center justify-between">
    <span class="text-sm font-medium">Rapoo VT2 Max</span>
    <span class="flex items-center gap-1.5 text-xs {vt2.connected ? 'text-success' : 'text-warning'}">
      <span class="relative flex size-2">
        {#if !vt2.connected}<span class="bg-warning absolute inline-flex size-full animate-ping rounded-full opacity-75"></span>{/if}
        <span class="relative inline-flex size-2 rounded-full {vt2.connected ? 'bg-success' : 'bg-warning'}"></span>
      </span>
      {t(vt2.connected ? (st?.wired ? "vt2.card.wired" : "vt2.card.wireless") : asleep ? "vt2.card.asleep" : "vt2.card.searching")}
    </span>
  </div>
  <Separator class="my-3" />
  <div class="flex items-center justify-between text-xs">
    <span class="text-muted-foreground flex items-center gap-1.5"><BatteryGlyph level={battery} charging={st?.charging} /><span class={batState === "low" ? "text-destructive" : ""}>{t(batState === "charging" ? "vt2.card.charging" : batState === "low" ? "vt2.card.low" : "vt2.card.battery")}</span></span>
    <span class="font-mono font-medium {batState === 'low' ? 'text-destructive' : ''}">{battery == null ? "—" : `${battery}%`}</span>
  </div>
  <Progress value={battery ?? 0} class="mt-2 h-1.5 {BATTERY_BAR[batState]}" />
  {#if vt2.serial}
    <div class="text-muted-foreground mt-3 font-mono text-[11px]">{t("vt2.card.serial", { serial: vt2.serial })}</div>
  {/if}
</ContextMenu.Trigger>
{#if COLORS.length}
  <ContextMenu.Portal>
    <ContextMenu.Content class="bg-popover text-popover-foreground ring-foreground/10 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 z-50 min-w-44 rounded-lg p-1 shadow-md ring-1 duration-100">
      <div class="text-muted-foreground px-1.5 py-1 text-xs">{t("vt2.card.color")}</div>
      <ContextMenu.RadioGroup value={String(prefs.color)} onValueChange={(v) => setColor(+v)}>
        {#each COLORS as c (c.key)}
          <ContextMenu.RadioItem value={String(c.key)} class="data-highlighted:bg-accent data-highlighted:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-md py-1 pr-8 pl-1.5 text-sm outline-hidden select-none">
            {#snippet children({ checked })}
              <span class="size-3 rounded-full border border-white/25" style="background:{c.hex}"></span>
              {c.name}
              {#if checked}<CheckIcon class="absolute right-2 size-4" />{/if}
            {/snippet}
          </ContextMenu.RadioItem>
        {/each}
      </ContextMenu.RadioGroup>
    </ContextMenu.Content>
  </ContextMenu.Portal>
{/if}
</ContextMenu.Root>
