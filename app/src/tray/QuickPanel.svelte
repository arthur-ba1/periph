<script>
  // Left click on a tray icon: the device, its battery and its quick settings (QuickItems).
  import Icon from "$lib/icons/Icon.svelte";
  import Battery, { batteryState, BATTERY_BAR, BATTERY_TONE } from "$lib/icons/Battery.svelte";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import { t } from "$lib/i18n.svelte.js";
  import QuickItems from "./QuickItems.svelte";
  import { KIND_ICON, command, openDevice } from "./api.js";

  let { panel } = $props();
  let device = $derived(panel.devices.find((d) => d.id === panel.device));
  let refresh = $derived(device?.items.find((i) => i.type === "action" && i.id === "refresh"));
  let items = $derived(device?.items.filter((i) => i !== refresh) ?? []);
  let battery = $derived(device?.battery);
  let state = $derived(battery ? batteryState(battery.percent, battery.charging, battery.percent >= 100) : null);
</script>

<div class="flex flex-col">
  {#if device}
    <header class="flex items-center gap-3 px-4 pt-4 pb-3">
      <div class="bg-muted grid size-10 shrink-0 place-items-center rounded-xl">
        <Icon name={KIND_ICON[device.kind] ?? "logo"} size={22} />
      </div>
      <div class="flex min-w-0 flex-1 flex-col">
        <span class="truncate text-sm font-semibold">{device.name}</span>
        <span class="flex items-center gap-1.5 text-xs {device.connected ? 'text-success' : 'text-muted-foreground'}">
          <span class="size-1.5 rounded-full {device.connected ? 'bg-success' : 'bg-muted-foreground/50'}"></span>
          {device.connected ? t("tray.connected") : t("tray.searching")}
        </span>
      </div>
      {#if refresh}
        <button
          class="text-muted-foreground hover:bg-accent hover:text-foreground grid size-8 place-items-center rounded-md transition-colors"
          title={refresh.label}
          aria-label={refresh.label}
          onclick={() => command(device.id, refresh.id)}
        >
          <Icon name="refresh" size={16} />
        </button>
      {/if}
    </header>

    {#if battery}
      <div class="mx-4 mb-3 rounded-xl border bg-card px-3.5 py-3">
        <div class="flex items-end justify-between">
          <span class="flex items-center gap-2 text-xs font-medium {BATTERY_TONE[state] || 'text-muted-foreground'}">
            <Battery level={battery.percent} charging={battery.charging} full={battery.percent >= 100} size={20} />
            {battery.charging ? t("tray.charging") : t("tray.battery")}
          </span>
          <span class="tabular text-3xl leading-none font-semibold tracking-tight {BATTERY_TONE[state]}">
            {battery.percent}<span class="text-muted-foreground ml-0.5 text-base font-medium">%</span>
          </span>
        </div>
        <Progress value={battery.percent} class="mt-3 h-1.5 {BATTERY_BAR[state]}" />
      </div>
    {/if}

    {#if items.length}
      <div class="border-t px-4 py-3">
        <QuickItems {device} {items} />
      </div>
    {/if}

    <footer class="border-t p-1.5">
      <button class="hover:bg-accent flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-sm font-medium transition-colors" onclick={() => openDevice(device.id)}>
        <Icon name="external" size={16} class="text-muted-foreground" />
        {t("tray.openDevice")}
      </button>
    </footer>
  {:else}
    <!-- the plain Periph icon: nothing connected -->
    <header class="flex items-center gap-3 px-4 pt-4 pb-3">
      <div class="bg-foreground text-background grid size-10 shrink-0 place-items-center rounded-xl"><Icon name="logo" size={22} strokeWidth={2} /></div>
      <div class="flex min-w-0 flex-col">
        <span class="text-sm font-semibold">Periph</span>
        <span class="text-muted-foreground text-xs">{t("tray.noDevices")}</span>
      </div>
    </header>
    <div class="flex flex-col gap-0.5 border-t p-1.5">
      {#each panel.devices as d}
        <div class="text-muted-foreground flex h-9 items-center gap-2.5 px-2.5 text-sm">
          <Icon name={KIND_ICON[d.kind] ?? "logo"} size={16} />
          <span class="text-foreground min-w-0 flex-1 truncate">{d.name}</span>
          <span class="text-xs">{t("tray.searching")}</span>
        </div>
      {/each}
    </div>
    <footer class="border-t p-1.5">
      <button class="hover:bg-accent flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-sm font-medium transition-colors" onclick={() => openDevice(null)}>
        <Icon name="external" size={16} class="text-muted-foreground" />
        {t("tray.open")}
      </button>
    </footer>
  {/if}
</div>
