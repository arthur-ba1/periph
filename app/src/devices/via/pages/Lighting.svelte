<script>
  // Lighting page generated from the VIA definition "menus" (range / dropdown / color / toggle).
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import KeyboardView, { fitUnit } from "../keyboard-view.svelte";
  import { kbd, sendKbd } from "../store.svelte.js";

  let def = $derived(kbd.definition);

  // Leaf groups = objects whose `content` is a list of controls (have `type`).
  function groupsOf(node, out = []) {
    if (Array.isArray(node)) node.forEach((n) => groupsOf(n, out));
    else if (node && typeof node === "object") {
      if (Array.isArray(node.content) && node.content.some((c) => c && typeof c === "object" && c.type)) out.push(node);
      else if (node.content) groupsOf(node.content, out);
    }
    return out;
  }
  let groups = $derived(def ? groupsOf(def.menus ?? []) : []);

  const key = (c) => `${c.content[1]}:${c.content[2]}`;
  const num = (bytes) => (bytes?.length === 2 && false ? 0 : bytes?.[0] ?? 0);
  function value(ctrl) {
    const b = kbd.values[key(ctrl)] ?? [];
    if (ctrl.type === "color") return b;
    return (ctrl.options?.[1] ?? 0) > 255 ? ((b[0] ?? 0) << 8) | (b[1] ?? 0) : b[0] ?? 0;
  }
  function write(ctrl, v) {
    const big = (ctrl.options?.[1] ?? 0) > 255;
    const data = ctrl.type === "color" ? v : big ? [v >> 8, v & 0xff] : [v];
    kbd.values[key(ctrl)] = data;
    sendKbd({ type: "setValue", channel: ctrl.content[1], id: ctrl.content[2], data, save: true });
  }

  // showIf: "{id_name} != 0" style expressions joined by && / ||
  function visible(ctrl) {
    if (!ctrl.showIf) return true;
    const byName = {};
    for (const g of groups) for (const c of g.content) if (c.content) byName[c.content[0]] = value(c);
    return ctrl.showIf.split("||").some((or) =>
      or.split("&&").every((cond) => {
        const m = cond.trim().match(/^\{(\w+)\}\s*(==|!=|>=|<=|>|<)\s*(\d+)$/);
        if (!m) return true;
        const a = Number(byName[m[1]] ?? 0), b = Number(m[3]);
        return { "==": a === b, "!=": a !== b, ">": a > b, "<": a < b, ">=": a >= b, "<=": a <= b }[m[2]];
      }),
    );
  }

  const PT = {
    "All Off": "Desligado", SOLID_COLOR: "Cor sólida", BREATHING: "Respiração", CYCLE_ALL: "Ciclo de cores",
    CYCLE_LEFT_RIGHT: "Ciclo esquerda → direita", CYCLE_UP_DOWN: "Ciclo cima → baixo", RAINBOW_MOVING_CHEVRON: "Arco-íris em V",
    CYCLE_OUT_IN: "Ciclo de fora para dentro", CYCLE_OUT_IN_DUAL: "Ciclo duplo de fora para dentro", CYCLE_PINWHEEL: "Cata-vento",
    CYCKE_SPIRAL: "Espiral", CYCLE_SPIRAL: "Espiral", DUAL_BEACON: "Farol duplo", RAINBOW_BEACON: "Farol arco-íris",
    RAINDROPS: "Gotas de chuva", TYPING_HEATMAP: "Mapa de calor da digitação", SOLID_REACTIVE_SIMPLE: "Reativo simples",
    SOLID_REACTIVE: "Reativo", SOLID_REACTIVE_CROSS: "Reativo em cruz", MATRIX_MULTISPLASH: "Respingos",
  };
  const pretty = (s) => PT[s] ?? s.toLowerCase().replace(/_/g, " ").replace(/^\w/, (c) => c.toUpperCase());

  // live preview from the first color/brightness/effect controls found
  let all = $derived(groups.flatMap((g) => g.content));
  let colorCtrl = $derived(all.find((c) => c.type === "color"));
  let effectCtrl = $derived(all.find((c) => c.type === "dropdown"));
  let brightCtrl = $derived(all.find((c) => c.type === "range" && /bright/i.test(c.content?.[0] ?? "")));
  // local drafts so sliders move smoothly; committed on release
  let drafts = $state({});
  let boxW = $state(700);
  const draft = (c) => drafts[key(c)] ?? value(c);

  // live preview: uses the slider drafts, so brightness / saturation / speed react while dragging
  let speedCtrl = $derived(all.find((c) => c.type === "range" && /speed/i.test(c.content?.[0] ?? "")));
  let light = $derived.by(() => {
    if (!effectCtrl) return null;
    const effect = (effectCtrl.options.find((o) => o[1] === draft(effectCtrl)) ?? [null])[0];
    const [h, s] = colorCtrl ? draft(colorCtrl) : [0, 0];
    const v = brightCtrl ? draft(brightCtrl) / (brightCtrl.options?.[1] ?? 255) : 1;
    const speed = speedCtrl ? draft(speedCtrl) / (speedCtrl.options?.[1] ?? 255) : 0.5;
    return { effect, h: h ?? 0, s: s ?? 0, v, speed };
  });
  // QMK HSV at full value -> css color (saturation 0 = white light)
  const swatch = (h, s) => `hsl(${Math.round(((h ?? 0) / 255) * 360)} 100% ${(100 - 50 * ((s ?? 0) / 255)).toFixed(1)}%)`;
</script>

{#if def}
  <PageHeader title="Iluminação" description="Brilho, efeito, velocidade e cor do RGB. Cada mudança é salva no teclado." />

  <div class="grid gap-4">
    <Card.Root>
      <Card.Content class="py-2"><div bind:clientWidth={boxW}>
        <KeyboardView {def} keymap={kbd.keymap[0]} encoders={kbd.encoders[0]} unit={fitUnit(def, boxW, 26, 40)} {light} /></div>
      </Card.Content>
    </Card.Root>

    {#each groups as g}
      <Card.Root>
        <Card.Header><Card.Title>{g.label === "Backlight" ? "Retroiluminação" : g.label}</Card.Title></Card.Header>
        <Card.Content class="grid gap-6">
          {#each g.content.filter(visible) as c (key(c))}
            <div class="grid gap-2.5">
              <div class="flex items-center justify-between text-sm">
                <span class="font-medium">{({ Brightness: "Brilho", Effect: "Efeito", "Effect Speed": "Velocidade do efeito", Color: "Cor" })[c.label] ?? c.label}</span>
                {#if c.type === "range"}<span class="text-muted-foreground font-mono text-xs">{draft(c)}</span>{/if}
              </div>

              {#if c.type === "range"}
                <Slider type="single" min={c.options?.[0] ?? 0} max={c.options?.[1] ?? 255} step={1}
                  value={draft(c)} onValueChange={(v) => (drafts[key(c)] = v)} onValueCommit={(v) => { delete drafts[key(c)]; write(c, v); }} />
              {:else if c.type === "dropdown"}
                <Select.Root type="single" value={String(value(c))} onValueChange={(v) => v !== undefined && write(c, +v)}>
                  <Select.Trigger class="w-80">{pretty((c.options.find((o) => o[1] === value(c)) ?? ["?"])[0])}</Select.Trigger>
                  <Select.Content class="max-h-72">
                    {#each c.options as [label, v]}
                      <Select.Item value={String(v)} label={pretty(label)}>{pretty(label)}</Select.Item>
                    {/each}
                  </Select.Content>
                </Select.Root>
              {:else if c.type === "toggle"}
                <Switch checked={!!value(c)} onCheckedChange={(v) => write(c, v ? 1 : 0)} />
              {:else if c.type === "color"}
                {@const hs = drafts[key(c)] ?? value(c)}
                <div class="flex items-center gap-5">
                  <div class="border-border size-14 shrink-0 rounded-xl border shadow-lg transition-colors"
                    style="background:{swatch(hs[0], hs[1])}; opacity:{0.35 + 0.65 * (light?.v ?? 1)}"></div>
                  <div class="grid flex-1 gap-3">
                    <div class="grid gap-1.5">
                      <span class="text-muted-foreground text-xs">Matiz</span>
                      <div class="hue h-2 rounded-full"></div>
                      <Slider type="single" min={0} max={255} step={1} value={hs[0] ?? 0}
                        onValueChange={(v) => (drafts[key(c)] = [v, hs[1] ?? 0])} onValueCommit={(v) => { delete drafts[key(c)]; write(c, [v, hs[1] ?? 0]); }} />
                    </div>
                    <div class="grid gap-1.5">
                      <span class="text-muted-foreground text-xs">Saturação</span>
                      <Slider type="single" min={0} max={255} step={1} value={hs[1] ?? 0}
                        onValueChange={(v) => (drafts[key(c)] = [hs[0] ?? 0, v])} onValueCommit={(v) => { delete drafts[key(c)]; write(c, [hs[0] ?? 0, v]); }} />
                    </div>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </Card.Content>
      </Card.Root>
    {/each}
  </div>
{/if}

<style>
  .hue {
    background: linear-gradient(90deg, hsl(0 90% 55%), hsl(60 90% 55%), hsl(120 90% 55%), hsl(180 90% 55%), hsl(240 90% 55%), hsl(300 90% 55%), hsl(360 90% 55%));
  }
</style>
