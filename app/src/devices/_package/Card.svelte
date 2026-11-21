<script>
  // Sidebar card of a package device: name, connection, battery (from tray.battery).
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import BatteryGlyph, { batteryState, BATTERY_BAR } from "$lib/icons/Battery.svelte";
  import { devices } from "$lib/device-state.svelte.js";
  import { t } from "$lib/i18n.svelte.js";
  import { KIND_ICON, text } from "./module.js";

  let { pkg } = $props();

  let dev = $derived(devices[pkg.manifest.id]);
  let spec = $derived(pkg.device.tray?.battery);
  let level = $derived(spec && dev.status ? (dev.status[spec.level] ?? null) : null);
  let charging = $derived(!!(spec?.charging && dev.status?.[spec.charging]));
  let state = $derived(batteryState(level, charging));
</script>

<div class="border-sidebar-border bg-card rounded-lg border p-3">
  <div class="flex items-center gap-3">
    <Icon name={KIND_ICON[pkg.manifest.kind] ?? "logo"} size={22} class="text-muted-foreground" />
    <div class="flex min-w-0 flex-col">
      <span class="truncate text-sm font-medium">{text(pkg, "name", pkg.manifest.name)}</span>
      <span class="text-xs {dev.connected ? 'text-success' : 'text-warning'}">{dev.connected ? t("app.online") : t("app.searching")}</span>
    </div>
  </div>
  {#if spec}
    <Separator class="my-3" />
    <div class="flex items-center justify-between text-xs">
      <span class="text-muted-foreground flex items-center gap-1.5"><BatteryGlyph {level} {charging} />{t("packages.ui.battery")}</span>
      <span class="font-mono font-medium">{level == null ? "—" : `${level}%`}</span>
    </div>
    <Progress value={level ?? 0} class="mt-2 h-1.5 {BATTERY_BAR[state]}" />
  {/if}
  <div class="text-muted-foreground mt-3 text-[11px]">{t("packages.ui.package", { version: pkg.manifest.version })}</div>
</div>
