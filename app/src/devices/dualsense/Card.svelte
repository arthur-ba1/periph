<script>
  // Sidebar card: controller glyph, connection / emulation state, battery.
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import { ContextMenu } from "bits-ui";
  import CheckIcon from "@lucide/svelte/icons/check";
  import Controller3D from "./controller-3d.svelte";
  import BatteryGlyph, { batteryState, BATTERY_BAR } from "$lib/icons/Battery.svelte";
  import { ds, battery, prefs, setModel, MODELS } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  let st = $derived(ds.status);
  let bat = $derived(ds.connected ? battery(st?.batteryRaw) : null);
  let batState = $derived(bat ? batteryState(bat.percent, bat.charging, bat.full) : null);
  const BAT_LABEL = { charging: "charging", full: "full", low: "low", ok: "battery" };
  let emulating = $derived(ds.connected && !!st?.virtualActive);
  let stateKey = $derived(!ds.connected ? "dualsense.card.searching" : emulating ? "dualsense.card.emulating" : "dualsense.card.connected");
</script>

<!-- right-click: pick the controller colourway the 3D views show -->
<ContextMenu.Root>
<ContextMenu.Trigger class="border-sidebar-border bg-card block rounded-lg border p-3">
  <div class="relative mb-3 flex h-28 items-center justify-center overflow-hidden rounded-md bg-gradient-to-b from-white/[0.04] to-transparent">
    <Controller3D compact model={prefs.model} class="transition-opacity duration-500 {ds.connected ? '' : 'opacity-30 grayscale'}" cfg={ds.config} lights={ds.connected ? ds.status?.lights : null}>
      {#snippet fallback()}
        <div class="grid h-full place-items-center"><Icon name="gamepad" size={56} class={ds.connected ? (emulating ? "text-success" : "text-foreground") : "text-muted-foreground opacity-40"} /></div>
      {/snippet}
    </Controller3D>
  </div>
  <div class="flex items-center justify-between">
    <span class="text-sm font-medium">DualSense</span>
    <span class="flex items-center gap-1.5 text-xs {ds.connected ? 'text-success' : 'text-warning'}">
      <span class="relative flex size-2">
        {#if !ds.connected}<span class="bg-warning absolute inline-flex size-full animate-ping rounded-full opacity-75"></span>{/if}
        <span class="relative inline-flex size-2 rounded-full {ds.connected ? 'bg-success' : 'bg-warning'}"></span>
      </span>
      {t(stateKey)}
    </span>
  </div>
  <Separator class="my-3" />
  <div class="flex items-center justify-between text-xs">
    <span class="text-muted-foreground flex items-center gap-1.5">
      <BatteryGlyph level={bat?.percent ?? null} charging={bat?.charging} full={bat?.full} />
      <span class={batState === "low" ? "text-destructive" : ""}>{t(`dualsense.card.${BAT_LABEL[batState] ?? "battery"}`)}</span>
    </span>
    <span class="font-mono font-medium {batState === 'low' ? 'text-destructive' : ''}">{bat == null ? "—" : `${bat.percent}%`}</span>
  </div>
  <Progress value={bat?.percent ?? 0} class="mt-2 h-1.5 {BATTERY_BAR[batState]}" />
</ContextMenu.Trigger>
<ContextMenu.Portal>
  <ContextMenu.Content class="bg-popover text-popover-foreground ring-foreground/10 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 z-50 min-w-44 rounded-lg p-1 shadow-md ring-1 duration-100">
    <div class="text-muted-foreground px-1.5 py-1 text-xs">{t("dualsense.card.model")}</div>
    <ContextMenu.RadioGroup value={prefs.model} onValueChange={setModel}>
      {#each MODELS as m (m)}
        <ContextMenu.RadioItem value={m} class="data-highlighted:bg-accent data-highlighted:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-md py-1 pr-8 pl-1.5 text-sm outline-hidden select-none">
          {#snippet children({ checked })}
            <span class="size-3 rounded-full border border-white/25 {m === 'midnight' ? 'bg-[#26272c]' : 'bg-[#ece8f0]'}"></span>
            {t(`dualsense.card.models.${m}`)}
            {#if checked}<CheckIcon class="absolute right-2 size-4" />{/if}
          {/snippet}
        </ContextMenu.RadioItem>
      {/each}
    </ContextMenu.RadioGroup>
  </ContextMenu.Content>
</ContextMenu.Portal>
</ContextMenu.Root>
