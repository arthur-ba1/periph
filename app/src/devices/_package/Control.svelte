<script>
  // The control for one package setting. Every change is sent as
  // { type: "set", id, index?, value }; the engine validates it again before writing.
  import * as Select from "$lib/components/ui/select/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import { send } from "$lib/device-state.svelte.js";
  import { usageName } from "$lib/hid.js";
  import { t } from "$lib/i18n.svelte.js";
  import { applyLocal, codecOf, controlOf, isReadOnly, num, settingOf, text } from "./module.js";

  let { pkg, setting: s, dev } = $props();

  const id = pkg.manifest.id;
  let cfg = $derived(dev.config);
  let kind = $derived(controlOf(pkg, s));
  let ro = $derived(isReadOnly(pkg, s));
  let unit = $derived(s.unit ? ` ${s.unit}` : "");
  let range = $derived(s.range && { min: num(s.range.min), max: num(s.range.max), step: num(s.range.step) || 1 });

  function set(value, index) {
    const cmd = index == null ? { type: "set", id: s.id, value } : { type: "set", id: s.id, index, value };
    applyLocal(cfg, cmd);
    send(id, cmd);
  }

  const fmtValue = (v, u = unit) => (v == null ? t("packages.ui.unknown") : typeof v === "boolean" ? String(v) : `${v}${u}`);
  const optionLabel = (v) => text(pkg, `settings.${s.id}.options.${v}`, fmtValue(v));
  const inRange = (v) => range && v >= range.min - 1e-6 && v <= range.max + 1e-6 && Math.abs((v - range.min) / range.step - Math.round((v - range.min) / range.step)) < 1e-6;

  // arrays: item labels from settings.<id>.items, or settings.<id>.item ("Stage {n}")
  function itemLabel(i) {
    const items = text(pkg, `settings.${s.id}.items`);
    if (Array.isArray(items) && items[i]) return items[i];
    return text(pkg, `settings.${s.id}.item`, t("packages.ui.item", { n: i + 1 }), { n: i + 1 });
  }
  let count = $derived.by(() => {
    const all = cfg[s.id]?.length ?? 0;
    const n = s.countFrom ? cfg[s.countFrom] : null;
    return typeof n === "number" ? Math.min(n, all) : all;
  });

  // index into another array (e.g. the active DPI stage)
  let target = $derived(s.indexOf ? settingOf(pkg, s.indexOf) : null);
  let targetCount = $derived.by(() => {
    if (!target) return 0;
    const all = cfg[target.id]?.length ?? 0;
    const n = target.countFrom ? cfg[target.countFrom] : null;
    return typeof n === "number" ? Math.min(n, all) : all;
  });
  // the stages list highlights the item another setting points at
  let pointer = $derived(pkg.device.settings.find((x) => x.indexOf === s.id));

  // slider keeps its own value while dragging; committed on release
  let slide = $state(0);
  $effect(() => {
    if (kind === "slider" && typeof cfg[s.id] === "number") slide = cfg[s.id];
  });
  const decimals = (step) => (String(step).split(".")[1] ?? "").length;

  // codec values ({ kind, value })
  let codec = $derived(codecOf(pkg, s));
  const variant = (k) => codec?.variants.find((v) => v.kind === k);
  const kindLabel = (k) => text(pkg, `codecs.${s.type}.${k}`, k);
  const writable = (k) => {
    const v = variant(k);
    return v && !v.readOnly;
  };
  const takesValue = (v) => v?.bytes.includes("{value}");
  function choices(v) {
    if (v.values) return v.values;
    if (v.range) {
      const out = [];
      for (let x = num(v.range.min); x <= num(v.range.max); x += num(v.range.step) || 1) out.push(x);
      return out;
    }
    return [];
  }
  const valueLabel = (v, x) =>
    v.display === "hidKey" ? usageName(x) : text(pkg, `codecs.${s.type}.values.${v.kind}.${x}`, String(x));
  function setKind(i, k) {
    const v = variant(k);
    if (!v) return;
    set(takesValue(v) ? { kind: k, value: choices(v)[0] } : { kind: k }, i);
  }
</script>

{#if kind === "switch"}
  <Switch checked={!!cfg[s.id]} onCheckedChange={(v) => set(v)} />
{:else if kind === "segmented"}
  <ToggleGroup.Root type="single" variant="outline" value={String(cfg[s.id])} onValueChange={(v) => v && set(s.values.map((m) => m.value).find((x) => String(x) === v))}>
    {#each s.values as m}
      <ToggleGroup.Item value={String(m.value)} class="px-3 font-mono text-xs">{optionLabel(m.value)}</ToggleGroup.Item>
    {/each}
  </ToggleGroup.Root>
{:else if kind === "select"}
  <Select.Root type="single" value={String(cfg[s.id])} onValueChange={(v) => v && set(s.values.map((m) => m.value).find((x) => String(x) === v))}>
    <Select.Trigger class="w-48">{optionLabel(cfg[s.id])}</Select.Trigger>
    <Select.Content>
      {#each s.values as m}
        <Select.Item value={String(m.value)} label={optionLabel(m.value)}>{optionLabel(m.value)}</Select.Item>
      {/each}
    </Select.Content>
  </Select.Root>
{:else if kind === "slider" && range}
  <div class="flex w-64 items-center gap-3">
    <Slider type="single" min={range.min} max={range.max} step={range.step} bind:value={slide} onValueCommit={(v) => v !== cfg[s.id] && set(+v.toFixed(decimals(range.step)))} />
    <span class="w-20 text-right font-mono text-sm">{slide.toFixed(decimals(range.step))}{unit}</span>
  </div>
{:else if kind === "number" && range}
  <Input type="number" class="w-32 font-mono" min={range.min} max={range.max} step={range.step} value={cfg[s.id]}
    onchange={(e) => (inRange(+e.currentTarget.value) ? set(+e.currentTarget.value) : (e.currentTarget.value = cfg[s.id]))} />
{:else if kind === "index"}
  <ToggleGroup.Root type="single" variant="outline" value={String(cfg[s.id])} onValueChange={(v) => v && set(+v)}>
    {#each Array.from({ length: targetCount }, (_, i) => i) as i}
      <ToggleGroup.Item value={String(i)} class="px-3 font-mono text-xs">{fmtValue(cfg[target.id][i], "")}</ToggleGroup.Item>
    {/each}
  </ToggleGroup.Root>
{:else if kind === "stages" || kind === "list"}
  <div class="grid gap-2">
    {#each Array.from({ length: count }, (_, i) => i) as i}
      {@const active = pointer && cfg[pointer.id] === i}
      <div class="flex items-center justify-between gap-4 rounded-md px-2 py-1 {active ? 'bg-accent' : ''}">
        <span class="text-sm {active ? 'font-medium' : 'text-muted-foreground'}">{itemLabel(i)}</span>
        {#if ro || !range}
          <span class="font-mono text-sm">{fmtValue(cfg[s.id][i])}</span>
        {:else}
          <div class="flex items-center gap-2">
            <Input type="number" class="w-28 text-right font-mono" min={range.min} max={range.max} step={range.step} value={cfg[s.id][i]}
              onchange={(e) => (inRange(+e.currentTarget.value) ? set(+e.currentTarget.value, i) : (e.currentTarget.value = cfg[s.id][i]))} />
            {#if s.unit}<span class="text-muted-foreground w-8 text-xs">{s.unit}</span>{/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>
{:else if kind === "actions" && codec}
  <div class="grid">
    {#each cfg[s.id] as cur, i}
      {@const v = variant(cur?.kind)}
      {#if i > 0}<Separator />{/if}
      <div class="flex items-center justify-between gap-4 py-2">
        <span class="text-sm">{itemLabel(i)}</span>
        {#if ro || !writable(cur?.kind)}
          <span class="text-muted-foreground flex items-center gap-1.5 text-sm" title={t("packages.ui.locked")}>
            <Icon name="lock" size={14} />{v ? kindLabel(cur.kind) : t("packages.ui.unknown")}{cur?.value != null ? ` · ${cur.value}` : ""}
          </span>
        {:else}
          <div class="flex items-center gap-2">
            <Select.Root type="single" value={cur.kind} onValueChange={(k) => k && setKind(i, k)}>
              <Select.Trigger class="w-40">{kindLabel(cur.kind)}</Select.Trigger>
              <Select.Content>
                {#each codec.variants.filter((x) => !x.readOnly) as x}
                  <Select.Item value={x.kind} label={kindLabel(x.kind)}>{kindLabel(x.kind)}</Select.Item>
                {/each}
              </Select.Content>
            </Select.Root>
            {#if takesValue(v)}
              <Select.Root type="single" value={String(cur.value)} onValueChange={(x) => x && set({ kind: cur.kind, value: +x }, i)}>
                <Select.Trigger class="w-40">{valueLabel(v, cur.value)}</Select.Trigger>
                <Select.Content>
                  {#each choices(v) as x}
                    <Select.Item value={String(x)} label={valueLabel(v, x)}>{valueLabel(v, x)}</Select.Item>
                  {/each}
                </Select.Content>
              </Select.Root>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>
{:else}
  <span class="font-mono text-sm">{fmtValue(cfg[s.id])}</span>
{/if}
