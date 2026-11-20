<script>
  // Right click on a tray icon: open the app (or a device), each device's quick settings
  // (expanded for the clicked icon's device, a chevron for the others), start with Windows, quit.
  // Arrow keys move between items, Enter activates, Escape closes (Flyout.svelte).
  import { onMount } from "svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import Battery, { batteryState, BATTERY_TONE } from "$lib/icons/Battery.svelte";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { t } from "$lib/i18n.svelte.js";
  import QuickItems from "./QuickItems.svelte";
  import { KIND_ICON, openDevice, quit, setAutostart } from "./api.js";

  let { panel } = $props();
  let connected = $derived(panel.devices.filter((d) => d.connected));
  let menu = $state();
  let expanded = $state(panel.device ? { [panel.device]: true } : {});
  const quick = (d) => d.items.filter((i) => !(i.type === "action" && i.id === "refresh"));

  function toggleAutostart() {
    panel.autostart = !panel.autostart;
    setAutostart(panel.autostart);
  }

  function onkeydown(e) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const items = [...menu.querySelectorAll("[role=menuitem], [role=menuitemcheckbox]")];
    const i = items.indexOf(document.activeElement);
    const next = e.key === "ArrowDown" ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
    items[i < 0 ? 0 : next]?.focus();
  }

  onMount(() => menu?.focus());

  const row = "flex h-8 w-full items-center gap-2.5 rounded-md px-2.5 text-left text-[13px] outline-none transition-colors hover:bg-accent focus-visible:bg-accent";
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div bind:this={menu} role="menu" tabindex="-1" class="flex flex-col p-1.5 outline-none" {onkeydown}>
  <button role="menuitem" class="{row} font-semibold" onclick={() => openDevice(null)}>
    <span class="bg-foreground text-background grid size-4 place-items-center rounded-[4px]"><Icon name="logo" size={12} strokeWidth={2.2} /></span>
    {t("tray.open")}
  </button>

  {#if connected.length}
    <div class="bg-border mx-1 my-1 h-px"></div>
    {#each connected as d}
      {@const state = d.battery ? batteryState(d.battery.percent, d.battery.charging, d.battery.percent >= 100) : null}
      {@const items = quick(d)}
      <div class="flex items-center gap-0.5">
        <button role="menuitem" class="{row} min-w-0 flex-1" onclick={() => openDevice(d.id)} title={d.header}>
          <Icon name={KIND_ICON[d.kind] ?? "logo"} size={16} class="text-muted-foreground" />
          <span class="min-w-0 flex-1 truncate">{d.name}</span>
          {#if d.battery}
            <span class="tabular flex items-center gap-1 text-xs {BATTERY_TONE[state] || 'text-muted-foreground'}">
              <Battery level={d.battery.percent} charging={d.battery.charging} full={d.battery.percent >= 100} size={16} />
              {d.battery.percent}%
            </span>
          {/if}
        </button>
        {#if items.length}
          <button
            role="menuitem"
            aria-expanded={!!expanded[d.id]}
            aria-label={d.name}
            class="text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:bg-accent grid size-8 shrink-0 place-items-center rounded-md outline-none transition-colors"
            onclick={() => (expanded[d.id] = !expanded[d.id])}
          >
            <Icon name="chevron" size={14} class="transition-transform duration-200 {expanded[d.id] ? 'rotate-180' : ''}" />
          </button>
        {/if}
      </div>
      {#if items.length && expanded[d.id]}
        <div class="mx-1 mt-0.5 mb-1.5 rounded-lg border bg-card px-2.5 py-2">
          <QuickItems device={d} {items} width={236} compact />
        </div>
      {/if}
    {/each}
  {/if}

  <div class="bg-border mx-1 my-1 h-px"></div>
  <button role="menuitemcheckbox" aria-checked={panel.autostart} class={row} onclick={toggleAutostart}>
    <span class="min-w-0 flex-1 truncate">{t("tray.autostart")}</span>
    <Switch checked={panel.autostart} size="sm" tabindex={-1} class="pointer-events-none" />
  </button>
  <button role="menuitem" class="{row} hover:text-destructive focus-visible:text-destructive" onclick={quit}>
    <Icon name="power" size={16} />
    {t("tray.quit")}
  </button>
</div>
