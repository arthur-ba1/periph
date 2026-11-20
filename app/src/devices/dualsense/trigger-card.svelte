<script>
  // One adaptive trigger (L2 or R2), or both at once when linked: mode, parameters, zone
  // preview and the override switch.
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import OverrideNote from "./override-note.svelte";
  import { ds, send, flashPart, partOf, TRIGGER_MODES, TRIGGER_PARAMS, CUSTOM_MODES, withParam, zonesOf } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  /** section: "leftTrigger" | "rightTrigger" (what is shown); linked: edits go to both */
  let { section, linked = false } = $props();

  let trig = $derived(ds.config[section]);
  let effect = $derived(trig.effect);
  let kind = $derived(effect?.kind ?? "none");
  /** Values while a slider is dragged (key -> value); sent on release. */
  let drafts = $state({});
  let shown = $derived(effect ? { ...effect, ...drafts } : null);
  let zones = $derived(zonesOf(shown));

  function commit(value) {
    drafts = {};
    for (const sec of linked ? ["leftTrigger", "rightTrigger"] : [section]) {
      ds.config[sec] = value;
      send(sec, value);
      flashPart(partOf(sec));
    }
  }

  function setMode(k) {
    if (!k || k === kind) return;
    const preset = TRIGGER_MODES.find((m) => m.kind === k);
    commit({ ...trig, effect: k === "none" ? null : structuredClone(preset) });
  }

  function setParam(key, v) {
    commit({ ...trig, effect: withParam(effect, key, v) });
  }

  function setZone(list, i, v) {
    const arr = [...effect[list]];
    arr[i] = v;
    if (arr.every((x) => x === 0)) return; // at least one zone must stay on
    commit({ ...trig, effect: { ...effect, [list]: arr } });
  }

  function setForce(i, v) {
    const forces = [...effect.forces];
    forces[i] = v;
    commit({ ...trig, effect: { ...effect, forces } });
  }

  const modeLabel = (k) => t(`dualsense.triggers.modes.${k}`);
  const hex = (n) => "0x" + n.toString(16).padStart(2, "0").toUpperCase();
</script>

<Card.Root class="min-w-0">
  <Card.Content class="grid gap-4">
    {#if kind !== "none"}
      <OverrideNote checked={trig.override} onchange={(v) => commit({ ...trig, override: v })} />
    {/if}
    <Select.Root type="single" value={kind} onValueChange={setMode}>
      <Select.Trigger class="w-full" aria-label={t("dualsense.triggers.mode")}>{kind === "none" ? t("dualsense.triggers.none") : modeLabel(kind)}</Select.Trigger>
      <Select.Content class="max-h-80">
        <Select.Item value="none" label={t("dualsense.triggers.none")}>{t("dualsense.triggers.none")}</Select.Item>
        <Select.Separator />
        {#each TRIGGER_MODES as m}
          <Select.Item value={m.kind} label={modeLabel(m.kind)}>{modeLabel(m.kind)}</Select.Item>
        {/each}
      </Select.Content>
    </Select.Root>

    {#if kind !== "none"}

      {#if zones}
        <!-- Travel from released (left) to fully pressed (right); bar height = strength. -->
        <div class="grid gap-1.5">
          <div class="flex h-10 items-end gap-1">
            {#each zones as z, i}
              {#if kind === "multiFeedback" || kind === "multiVibration"}
                {@const list = kind === "multiFeedback" ? "strengths" : "amplitudes"}
                <button
                  type="button"
                  class="bg-muted/40 relative h-full flex-1 overflow-hidden rounded-sm"
                  aria-label={t("dualsense.triggers.zone", { n: i, v: z })}
                  onclick={(e) => {
                    const r = e.currentTarget.getBoundingClientRect();
                    setZone(list, i, Math.max(0, Math.min(8, Math.round(((r.bottom - e.clientY) / r.height) * 8))));
                  }}
                >
                  <span class="bg-brand absolute inset-x-0 bottom-0" style:height="{(z / 8) * 100}%"></span>
                </button>
              {:else}
                <div class="bg-muted/40 relative h-full flex-1 overflow-hidden rounded-sm">
                  <span class="bg-brand absolute inset-x-0 bottom-0 transition-[height]" style:height="{(z / 8) * 100}%"></span>
                </div>
              {/if}
            {/each}
          </div>
          <div class="text-muted-foreground flex justify-between text-[10px]">
            <span>{t("dualsense.triggers.released")}</span><span>{t("dualsense.triggers.pressed")}</span>
          </div>
        </div>
      {/if}

      {#if TRIGGER_PARAMS[kind]}
      <div class="grid grid-cols-2 gap-x-5 gap-y-3.5">
      {#each TRIGGER_PARAMS[kind] as [key, min, max]}
        <div class="grid gap-2">
          <div class="flex justify-between text-sm">
            <span>{t(`dualsense.triggers.params.${key}`)}</span>
            <span class="text-muted-foreground font-mono text-xs">{shown[key]}</span>
          </div>
          <Slider type="single" {min} {max} step={1} value={shown[key]}
            onValueChange={(v) => (drafts = { ...drafts, [key]: v })} onValueCommit={(v) => setParam(key, v)} />
        </div>
      {/each}
      </div>
      {/if}

      {#if kind === "custom"}
        <div class="grid gap-2">
          <span class="text-sm">{t("dualsense.triggers.params.mode")}</span>
          <Select.Root type="single" value={String(effect.mode)} onValueChange={(v) => commit({ ...trig, effect: { ...effect, mode: +v } })}>
            <Select.Trigger class="w-full">{t(`dualsense.triggers.custom.${effect.mode}`)} <span class="text-muted-foreground ml-1 font-mono text-xs">{hex(effect.mode)}</span></Select.Trigger>
            <Select.Content>
              {#each CUSTOM_MODES as m}
                <Select.Item value={String(m)} label={t(`dualsense.triggers.custom.${m}`)}>{t(`dualsense.triggers.custom.${m}`)} <span class="text-muted-foreground ml-1 font-mono text-xs">{hex(m)}</span></Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
        </div>
        <div class="grid grid-cols-2 gap-x-5 gap-y-3">
        {#each effect.forces as f, i}
          <div class="grid gap-1.5">
            <div class="flex justify-between text-sm">
              <span>{t("dualsense.triggers.force", { n: i + 1 })}</span>
              <span class="text-muted-foreground font-mono text-xs">{drafts[`f${i}`] ?? f}</span>
            </div>
            <Slider type="single" min={0} max={255} step={1} value={drafts[`f${i}`] ?? f}
              onValueChange={(v) => (drafts = { ...drafts, [`f${i}`]: v })} onValueCommit={(v) => setForce(i, v)} />
          </div>
        {/each}
        </div>
      {/if}

    {/if}
  </Card.Content>
</Card.Root>
