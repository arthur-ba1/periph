<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import SettingRow from "$lib/components/setting-row.svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import BatteryGlyph, { batteryState } from "$lib/icons/Battery.svelte";
  import PageLayout from "../page-layout.svelte";
  import { ds, send, battery } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  let cfg = $derived(ds.config);
  let st = $derived(ds.status);
  let active = $derived(!!st?.virtualActive);
  let hidHide = $derived(st?.hidHide ?? "off");
  let bat = $derived(battery(st?.batteryRaw));
  let batState = $derived(bat ? batteryState(bat.percent, bat.charging, bat.full) : null);

  /** Optimistic update; the driver answers with the real config (and an error toast if it fails). */
  function set(key, value) {
    if (!cfg || cfg[key] === value) return;
    cfg[key] = value;
    send(key, value);
  }
</script>

{#if cfg}
  <PageLayout title={t("dualsense.emulation.title")} home="overview" virtual={active}>
    <div class="grid gap-3">
      <Card.Root class="gap-4 py-4 transition-[box-shadow,border-color] duration-500 {active ? 'border-success/40 shadow-[0_8px_30px_-12px_var(--color-success)]' : ''}">
        <Card.Content class="flex items-center gap-4 px-4">
          <div class="flex size-11 shrink-0 items-center justify-center rounded-xl border transition-colors {active ? 'border-success/40 bg-success/10 text-success' : 'bg-muted/40 text-muted-foreground'}">
            <Icon name="emulate" size={24} />
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="font-semibold">{t("dualsense.emulation.virtualTitle")}</span>
              <Badge variant="outline" class={active ? "border-success/40 text-success" : "text-muted-foreground"}>
                {t(active ? "dualsense.emulation.active" : "dualsense.emulation.inactive")}
              </Badge>
            </div>
          </div>
          <Switch checked={cfg.emulation} disabled={ds.busy} onCheckedChange={(v) => set("emulation", v)} aria-label={t("dualsense.emulation.virtualTitle")} />
        </Card.Content>
        <Card.Content class="grid grid-cols-3 gap-2 px-4">
          {#each [
            [t("dualsense.emulation.statBattery"), bat ? `${bat.percent}%` : "—", batState && batState !== "ok" ? t(`dualsense.card.${batState}`) : "", batState === "low" ? "text-destructive" : ""],
            [t("dualsense.emulation.statReports"), String(st?.outputReports ?? 0), "", ""],
            [t("dualsense.emulation.statHidHide"), t(`dualsense.emulation.hidHide.${hidHide}`), "", hidHide === "hiding" ? "text-success" : hidHide === "missing" ? "text-warning" : ""],
          ] as [label, value, note, tone], i}
            <div class="bg-muted/30 min-w-0 rounded-lg border px-3 py-2">
              <div class="text-muted-foreground truncate text-[11px]">{label}</div>
              <div class="mt-0.5 flex items-center gap-1.5 truncate font-mono text-base font-semibold tracking-tight {tone}">
                {#if i === 0}<BatteryGlyph level={bat?.percent ?? null} charging={bat?.charging} full={bat?.full} size={18} />{/if}
                {value}
              </div>
              {#if note}<div class="truncate text-[10px] {batState === 'low' && i === 0 ? 'text-destructive' : batState && i === 0 ? 'text-success' : 'text-muted-foreground'}" title={note}>{note}</div>{/if}
            </div>
          {/each}
        </Card.Content>
      </Card.Root>

      <Card.Root class="gap-0 py-1">
        <Card.Content class="divide-border divide-y px-4">
          <SettingRow title={t("dualsense.emulation.forward")}>
            <Switch checked={cfg.forwardFeedback} disabled={ds.busy} onCheckedChange={(v) => set("forwardFeedback", v)} />
          </SettingRow>
          <SettingRow title={t("dualsense.emulation.hide")}>
            <Switch checked={cfg.hidePhysical} disabled={ds.busy || hidHide === "missing"} onCheckedChange={(v) => set("hidePhysical", v)} />
          </SettingRow>
          {#if hidHide === "missing"}
            <div class="flex gap-3 py-3">
              <Icon name="warning" class="text-warning mt-0.5" />
              <div class="text-muted-foreground text-xs leading-relaxed">{t("dualsense.emulation.hidHideMissing")}</div>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>

    </div>
  </PageLayout>
{/if}
