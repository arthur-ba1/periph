<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import SettingRow from "$lib/components/setting-row.svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import BatteryGlyph, { batteryState } from "$lib/icons/Battery.svelte";
  import PageLayout from "../page-layout.svelte";
  import { ds, send, battery, prefs, setEmulationTarget } from "../store.svelte.js";
  import { EMULATION_MODELS } from "../emulation-models.js";
  import { t } from "$lib/i18n.svelte.js";

  let cfg = $derived(ds.config);
  let st = $derived(ds.status);
  let active = $derived(!!st?.virtualActive);
  let target = $derived(prefs.emulationTarget);
  let supported = $derived(EMULATION_MODELS[target].supported);
  let showDualSenseControls = $derived(supported || active || cfg?.emulation);
  const modelName = (id) => t(`dualsense.emulation.models.${id}`);
  let hidHide = $derived(st?.hidHide ?? "off");
  let bat = $derived(battery(st?.batteryRaw));
  let batState = $derived(bat ? batteryState(bat.percent, bat.charging, bat.full) : null);

  /** Optimistic update; the driver answers with the real config (and an error toast if it fails). */
  function set(key, value) {
    if (!cfg || cfg[key] === value) return;
    cfg[key] = value;
    send(key, value);
  }

  /** Who controls each part of the pad while emulating. `haptics` is the game's vibration
   *  (on/off: Periph has none of its own); stored as a bool, shown as game / off. */
  const FEEDBACK = [
    { key: "lights", icon: "lightbar", options: ["periph", "game"] },
    { key: "haptics", icon: "gamepad", options: ["game", "off"] },
    { key: "triggers", icon: "trigger", options: ["periph", "game"] },
  ];
  const fbValue = (key) => (key === "haptics" ? (cfg.feedback.haptics ? "game" : "off") : cfg.feedback[key]);
  function setFeedback(key, v) {
    if (!v || v === fbValue(key)) return;
    set("feedback", { ...cfg.feedback, [key]: key === "haptics" ? v === "game" : v });
  }
</script>

{#if cfg}
  <PageLayout title={t("dualsense.emulation.title")} home="overview" virtual={active} preview={supported ? null : target}>
    <div class="grid gap-3">
      <Card.Root class="gap-4 py-4 transition-[box-shadow,border-color] duration-500 {active ? 'border-success/40 shadow-[0_8px_30px_-12px_var(--color-success)]' : ''}">
        <Card.Content class="flex items-center gap-4 px-4">
          <div class="flex size-11 shrink-0 items-center justify-center rounded-xl border transition-colors {active ? 'border-success/40 bg-success/10 text-success' : 'bg-muted/40 text-muted-foreground'}">
            <Icon name="emulate" size={24} />
          </div>
          <div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-1">
            <span class="font-semibold">{t("dualsense.emulation.virtualTitle")}</span>
            <Badge variant="outline" class={active ? "border-success/40 text-success" : "text-muted-foreground"}>
              {active && !supported
                ? t("dualsense.emulation.activeModel", { model: modelName("dualsense") })
                : t(active ? "dualsense.emulation.active" : "dualsense.emulation.inactive")}
            </Badge>
          </div>
          <Switch checked={cfg.emulation} disabled={ds.busy || (!supported && !cfg.emulation)} onCheckedChange={(v) => (!v || supported) && set("emulation", v)} aria-label={t("dualsense.emulation.virtualTitle")} />
        </Card.Content>
        <Card.Content class="px-4">
          <fieldset class="grid min-w-0 grid-cols-3 gap-2">
            <legend class="text-muted-foreground mb-2 text-xs font-medium">{t("dualsense.emulation.target")}</legend>
            {#each Object.entries(EMULATION_MODELS) as [id, model] (id)}
              <label class="min-w-0 cursor-pointer">
                <input class="peer sr-only" type="radio" name="emulation-target" value={id} checked={target === id} onchange={() => setEmulationTarget(id)} />
                <span class="text-muted-foreground hover:bg-muted/50 hover:text-foreground peer-checked:border-brand/50 peer-checked:bg-brand/10 peer-checked:text-brand peer-checked:hover:bg-brand/15 peer-checked:hover:text-brand peer-focus-visible:ring-ring/50 flex h-full flex-col items-center gap-1.5 rounded-lg border px-1 py-2.5 transition-colors peer-focus-visible:ring-3">
                  <Icon name={model.icon} size={28} />
                  <span class="text-center text-[11px] leading-tight">{modelName(id)}</span>
                </span>
              </label>
            {/each}
          </fieldset>
          {#if !supported}
            <div role="status" class="border-warning/20 bg-warning/5 text-muted-foreground mt-3 flex items-start gap-2.5 rounded-lg border px-3 py-2.5 text-[13px]">
              <Icon name="warning" size={16} class="text-warning mt-0.5 shrink-0" />
              <span>{t("dualsense.emulation.previewOnly", { model: modelName(target) })}</span>
            </div>
          {/if}
        </Card.Content>
        {#if showDualSenseControls}
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
        {/if}
      </Card.Root>
      {#if showDualSenseControls}
        <Card.Root class="gap-0 py-1">
          <Card.Content class="px-4">
            <div class="pt-3.5 pb-1">
              <div class="text-sm font-medium">{t("dualsense.emulation.feedback.title")}</div>
              <div class="text-muted-foreground mt-0.5 text-[13px] leading-snug">{t("dualsense.emulation.feedback.desc")}</div>
            </div>
            <div class="divide-border divide-y">
              {#each FEEDBACK as f (f.key)}
                <div class="flex items-center justify-between gap-4 py-3">
                  <span class="flex items-center gap-2 text-sm"><Icon name={f.icon} size={16} class="text-muted-foreground" />{t(`dualsense.emulation.feedback.${f.key}`)}</span>
                  <ToggleGroup.Root type="single" variant="outline" size="sm" disabled={ds.busy} value={fbValue(f.key)} onValueChange={(v) => setFeedback(f.key, v)}>
                    {#each f.options as o (o)}
                      <ToggleGroup.Item value={o} class="min-w-20 px-3 text-xs">{t(`dualsense.emulation.feedback.${o}`)}</ToggleGroup.Item>
                    {/each}
                  </ToggleGroup.Root>
                </div>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root class="gap-0 py-1">
          <Card.Content class="divide-border divide-y px-4">
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
      {/if}
    </div>
  </PageLayout>
{/if}
