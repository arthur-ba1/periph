<script module>
  // Case margin around the key field, in key units (each side). Pages size `unit` with fitUnit().
  export const CASE_PAD = 0.42;
  export const fitUnit = (def, boxW, min, max) =>
    Math.max(min, Math.min(max, Math.floor(boxW / (def.keys.width + CASE_PAD * 2))));
</script>

<script>
  // VIA-style keyboard: white case, sculpted white keycaps, slight 3D tilt, rotary knob.
  // Keys show the keycode of the given layer.
  import { onMount } from "svelte";
  import { describe, keycodeFromEvent } from "./keycodes.js";
  import { t } from "$lib/i18n.svelte.js";
  import { prefs } from "./store.svelte.js";

  let {
    def,
    keymap = null, // [row][col] for the shown layer
    selected = null, // "row,col"
    onselect,
    unit = 44,
    // RGB preview: { effect: VIA option name, h, s: 0..255 (QMK HSV), v: 0..1 brightness, speed: 0..1 }
    light = null,
    duelColors = null, // [left, right] CSS colors for the color duel preview
    wordKeys = null, // indices in layout order for each step of the word preview
    tilt = true,
    encoders = null, // [encoder] = [ccw, cw] for the shown layer
  } = $props();

  // knob: wheel over it turns it (visual only) and shows what that direction does
  let turn = $state({}); // encoder -> degrees
  let flash = $state(null); // { enc, text }
  let flashTimer;
  const encLabel = (e, cw) => (encoders?.[e] ? describe(encoders[e][cw ? 1 : 0], def).short : cw ? "↻" : "↺");
  function wheel(ev, e) {
    ev.preventDefault();
    const cw = ev.deltaY < 0; // wheel up = clockwise (Vol+)
    turn[e] = (turn[e] ?? 0) + (cw ? 24 : -24);
    flash = { enc: e, text: `${cw ? "↻" : "↺"} ${encLabel(e, cw)}` };
    clearTimeout(flashTimer);
    flashTimer = setTimeout(() => (flash = null), 700);
  }
  const knobTitle = (e, d) =>
    encoders?.[e] ? t("via.keymap.knobTitle", { press: d.long, ccw: encLabel(e, false), cw: encLabel(e, true) }) : d.long;

  // ---------------- RGB preview ----------------
  // QMK colors are HSV with value = brightness. HSV(h, s, 1) == HSL(h, 100%, 100 - 50*s %),
  // so saturation 0 is white light (not grey) and brightness only scales the light's intensity.
  const FX = {
    SOLID_COLOR: "solid", BREATHING: "breathe", CYCLE_ALL: "all", CYCLE_LEFT_RIGHT: "lr", CYCLE_UP_DOWN: "ud",
    RAINBOW_MOVING_CHEVRON: "chevron", CYCLE_OUT_IN: "outin", CYCLE_OUT_IN_DUAL: "outin2", CYCLE_PINWHEEL: "pinwheel",
    CYCKE_SPIRAL: "spiral", CYCLE_SPIRAL: "spiral", DUAL_BEACON: "beacon2", RAINBOW_BEACON: "beacon", RAINDROPS: "rain",
    TYPING_HEATMAP: "heat", SOLID_REACTIVE_SIMPLE: "simple", SOLID_REACTIVE: "reactive", SOLID_REACTIVE_CROSS: "cross",
    MATRIX_MULTISPLASH: "splash",
  };
  const RAINBOW = new Set(["all", "lr", "ud", "chevron", "outin", "outin2", "pinwheel", "spiral", "beacon2", "beacon"]);
  const REACTIVE = new Set(["heat", "simple", "reactive", "cross", "splash"]);
  let duel = $derived(duelColors !== null);

  let fx = $derived.by(() => {
    if (duel) return "duel";
    if (wordKeys) return wordKeys.length ? "word" : null;
    if (!light || !light.effect || light.effect === "All Off" || light.v <= 0) return null;
    return FX[light.effect] ?? (/CYCLE|RAINBOW/.test(light.effect) ? "lr" : "solid");
  });
  let durMs = $derived.by(() => {
    const sp = light?.speed ?? 0.5;
    const sec = fx === "breathe" ? 6 - 4.6 * sp : fx === "rain" ? 5 - 3 * sp : REACTIVE.has(fx) ? 2.6 - 2 * sp : 12 - 10.5 * sp;
    return sec * 1000;
  });
  let lightVars = $derived.by(() => {
    if (!fx) return "";
    if (duel) return "--S:100%; --a:0.85;";
    if (fx === "word") return "--H:185; --L:50%; --S:100%; --a:0.9;";
    const a = Math.pow(light.v, 0.6); // perceptual: low brightness still reads, 0 is off
    return `--H:${Math.round((light.h / 255) * 360)}; --L:${(100 - 50 * (light.s / 255)).toFixed(1)}%; --S:${Math.round((light.s / 255) * 100)}%; --a:${a.toFixed(3)}; --dur:${Math.round(durMs)}ms;`;
  });

  let wordKey = $state(null);
  $effect(() => {
    const sequence = wordKeys;
    if (!sequence?.length) return;
    let index = 0;
    let timer;
    function playStep() {
      wordKey = sequence[index];
      timer = setTimeout(() => {
        wordKey = null;
        index = (index + 1) % sequence.length;
        timer = setTimeout(playStep, index === 0 ? 800 : 100);
      }, 380);
    }
    playStep();
    return () => {
      clearTimeout(timer);
      wordKey = null;
    };
  });

  const DUEL_CENTER = 0.5;
  const DUEL_STEP = 0.07;
  const DUEL_IDLE_MS = 600;
  const DUEL_RETURN_MS = 900;
  const keyCenterX = (k) => (k.x + k.w / 2) / def.keys.width;
  let boundary = $state(DUEL_CENTER);
  let returnTimer;
  let returnFrame;

  function stopReturn() {
    clearTimeout(returnTimer);
    cancelAnimationFrame(returnFrame);
  }

  $effect(() => {
    if (!duel) return;
    boundary = DUEL_CENTER;
    return stopReturn;
  });

  function duelHit(k) {
    stopReturn();
    const step = keyCenterX(k) < DUEL_CENTER ? DUEL_STEP : -DUEL_STEP;
    boundary = Math.max(0.18, Math.min(0.82, boundary + step));
    returnTimer = setTimeout(() => {
      const from = boundary;
      const start = performance.now();
      function returnToCenter(now) {
        const progress = Math.min(1, (now - start) / DUEL_RETURN_MS);
        const remaining = (1 - progress) ** 2;
        boundary = DUEL_CENTER + (from - DUEL_CENTER) * remaining;
        if (progress < 1) returnFrame = requestAnimationFrame(returnToCenter);
      }
      returnFrame = requestAnimationFrame(returnToCenter);
    }, DUEL_IDLE_MS);
  }

  const rnd = (i) => { const x = Math.sin(i * 12.9898 + 4.1) * 43758.5453; return x - Math.floor(x); };
  // per-key phase of the rainbow wave (0..1), from the key's centre in the layout
  function phase(k) {
    const W = def.keys.width, H = def.keys.height;
    const px = (k.x + k.w / 2) / W, py = (k.y + k.h / 2) / H;
    const dx = px - 0.5, dy = (py - 0.5) * (H / W);
    const dist = Math.hypot(dx, dy) / Math.hypot(0.5, 0.5 * (H / W));
    const ang = (Math.atan2(dy, dx) / (2 * Math.PI) + 1) % 1;
    switch (fx) {
      case "lr": return px;
      case "ud": return py;
      case "chevron": return Math.abs(px - 0.5) * 1.2 + py * 0.3;
      case "outin": return dist;
      case "outin2": return Math.min(Math.hypot(px - 0.25, dy), Math.hypot(px - 0.75, dy)) * 2;
      case "pinwheel": return ang;
      case "spiral": return ang + dist;
      case "beacon2": return ang * 2;
      case "beacon": return ang;
      default: return 0;
    }
  }
  function capLight(k, i) {
    if (!fx) return "";
    if (duelColors) {
      const blend = 0.5 + (boundary - keyCenterX(k)) / DUEL_STEP;
      const left = Math.max(0, Math.min(1, blend));
      return `--duel-color:color-mix(in oklch, ${duelColors[0]} ${(left * 100).toFixed(1)}%, ${duelColors[1]});`;
    }
    if (RAINBOW.has(fx)) return `--ph:${phase(k).toFixed(3)};`;
    if (fx === "rain") return `--rd:${(rnd(i) * 6).toFixed(2)}; --rh:${Math.round((rnd(i + 99) - 0.5) * 120)};`;
    return "";
  }

  // reactive effects: clicks in the preview or real key presses light the keys up
  let caseEl = $state();
  function hit(k) {
    if (!fx || !REACTIVE.has(fx) || !caseEl) return;
    const cx = k.x + k.w / 2, cy = k.y + k.h / 2;
    for (const o of def.keys.list) {
      const el = caseEl.querySelector(`[data-key="${o.row},${o.col}"]`);
      if (!el) continue;
      const d = Math.hypot(o.x + o.w / 2 - cx, o.y + o.h / 2 - cy);
      let delay = 0, peak = 1;
      if (o === k) delay = 0;
      else if (fx === "cross" && (Math.abs(o.y + o.h / 2 - cy) < 0.5 || Math.abs(o.x + o.w / 2 - cx) < 0.5)) { delay = d * 45; peak = Math.max(0.25, 1 - d / 8); }
      else if (fx === "splash" && d < 6) { delay = d * 70; peak = Math.max(0.2, 1 - d / 6); }
      else if (fx === "heat" && d < 1.6) peak = 0.45;
      else continue;
      const base = fx === "reactive" ? 0.55 : fx === "heat" ? 0 : 0.06;
      const frames = fx === "reactive" || fx === "splash"
        ? [{ "--k": peak, "--hs": fx === "splash" ? 0 : 140 }, { "--k": base, "--hs": fx === "splash" ? 300 : 0 }]
        : [{ "--k": peak }, { "--k": base }];
      el.animate(frames, { duration: fx === "heat" ? durMs * 2.5 : durMs, delay, easing: "cubic-bezier(0.2, 0.7, 0.3, 1)" });
    }
  }
  onMount(() => {
    const onKey = (e) => {
      if ((!duel && !REACTIVE.has(fx)) || e.repeat) return;
      if (duel) {
        if (e.defaultPrevented || e.isComposing || e.ctrlKey || e.altKey || e.metaKey) return;
        if (e.code === "Tab" || e.code === "Escape") return;
        if (e.target instanceof Element) {
          if (e.target.closest("input, textarea, select, [contenteditable], [role=slider], [role=listbox], [data-color-picker]")) return;
          if ((e.code === "Space" || e.code === "Enter") && e.target.closest("button, a, [role=button], [role=switch]")) return;
        }
      }
      const code = keycodeFromEvent(e);
      const k = code !== undefined && def.keys.list.find((o) => keymap?.[o.row]?.[o.col] === code);
      if (!k) return;
      if (duel) duelHit(k);
      else hit(k);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // legend font size (px) that fits the cap's top face: width-limited for long labels
  // legend size: measured with the real font so every legend fits its keycap whole (never clipped)
  let measureCtx;
  function textWidth(text) {
    if (typeof document === "undefined") return [...text].length * 0.7 * 100;
    measureCtx ??= document.createElement("canvas").getContext("2d");
    measureCtx.font = `650 100px ${getComputedStyle(document.body).fontFamily}`;
    return measureCtx.measureText(text).width * 1.04; // + bold/hinting slack
  }
  function fit(text, k) {
    // top face minus cap padding (0.06), side walls (0.11) and inner padding (0.07), each side
    const avail = k.w * unit - unit * 0.48;
    const byWidth = text ? (avail * 100) / textWidth(text) : Infinity;
    return Math.max(5, Math.min(unit * 0.27, byWidth)).toFixed(1);
  }
  const fitKnob = (text) => Math.max(6, Math.min(unit * 0.17, (unit * 0.5) / Math.max(1, [...text].length * 0.6))).toFixed(1);

  let mini = $derived(unit < 20); // sidebar thumbnail: no labels, no tilt
  let pad = $derived(Math.round(unit * CASE_PAD));

  // Plate wells overhang their key by WELL_E (key units). Where two wells form an inside corner
  // (the inverted T of the arrows, the step from Enter to Right Shift...) a fillet of radius
  // WELL_R rounds it, so clusters read as one rounded cut-out.
  const WELL_E = 0.05, WELL_R = 0.1;
  // Outer corners of a well are rounded only where they are a corner of the whole cluster; where
  // a neighbour continues the cut-out the corner stays square, so stacked keys (Home..End) read
  // as one unbroken well instead of pinched segments. Per key: "tl tr br bl" in px.
  const CORNER_R = 0.12;
  let wellRadius = $derived.by(() => {
    const keys = def.keys.list.filter((k) => k.encoder === null);
    const w = keys.map((k) => ({ x0: k.x - WELL_E, y0: k.y - WELL_E, x1: k.x + k.w + WELL_E, y1: k.y + k.h + WELL_E }));
    const inside = (px, py) => w.some((r) => r.x0 < px && px < r.x1 && r.y0 < py && py < r.y1);
    const e = 1e-3, R = `${(CORNER_R * unit).toFixed(1)}px`;
    const corner = (x, y, sx, sy) => (inside(x + sx * e, y + sy * e) || inside(x + sx * e, y - sy * e) || inside(x - sx * e, y + sy * e) ? "0" : R);
    const out = new Map();
    keys.forEach((k, i) => {
      const r = w[i];
      out.set(`${k.row},${k.col},${k.x}`, [corner(r.x0, r.y0, -1, -1), corner(r.x1, r.y0, 1, -1), corner(r.x1, r.y1, 1, 1), corner(r.x0, r.y1, -1, 1)].join(" "));
    });
    return out;
  });
  let fillets = $derived.by(() => {
    const w = def.keys.list.filter((k) => k.encoder === null)
      .map((k) => ({ x0: k.x - WELL_E, y0: k.y - WELL_E, x1: k.x + k.w + WELL_E, y1: k.y + k.h + WELL_E }));
    const inside = (px, py) => w.some((r) => r.x0 < px && px < r.x1 && r.y0 < py && py < r.y1);
    const out = new Map();
    const eps = 1e-3;
    for (const a of w) for (const b of w) {
      if (a === b) continue;
      for (const vx of [a.x0, a.x1]) for (const hy of [b.y0, b.y1]) {
        if (!(b.x0 < vx && vx < b.x1 && a.y0 < hy && hy < a.y1)) continue;
        const empty = [[-1, -1], [1, -1], [-1, 1], [1, 1]].filter(([sx, sy]) => !inside(vx + sx * eps, hy + sy * eps));
        if (empty.length !== 1) continue;
        const [sx, sy] = empty[0];
        out.set(`${vx.toFixed(3)},${hy.toFixed(3)}`, { x: sx < 0 ? vx - WELL_R : vx, y: sy < 0 ? hy - WELL_R : hy, sx, sy });
      }
    }
    return [...out.values()];
  });

  // KLE colors only separate alphas / mods / accents; all caps stay white, mods a touch greyer
  const tone = (c) => {
    const l = parseInt(c.slice(1, 3), 16);
    return l < 0x90 ? "accent" : l < 0xc0 ? "mod" : "alpha";
  };
</script>

<div class="stage" class:tilted={tilt && !mini} style="--u:{unit}px">
  <div
    bind:this={caseEl}
    class="case relative mx-auto select-none {fx ? `lit fx-${fx}` : ''}"
    class:mini
    class:black={prefs.color === "black"}
    class:solid={prefs.caps === "solid"}
    class:rainbow={fx && RAINBOW.has(fx)}
    style="width:{def.keys.width * unit + pad * 2}px; height:{def.keys.height * unit + pad * 2}px; --pad:{pad}px; {lightVars}"
  >
    <!-- plate shows only under the keys: adjacent wells merge into clusters, the gaps between clusters are case -->
    {#each def.keys.list.filter((k) => k.encoder === null) as k (k.row + "," + k.col + "," + k.x)}
      <div class="plate" style="left:{pad + k.x * unit}px; top:{pad + k.y * unit}px; width:{k.w * unit}px; height:{k.h * unit}px; border-radius:{wellRadius.get(`${k.row},${k.col},${k.x}`)}"></div>
    {/each}
    {#each fillets as f}
      <!-- the circle sits on the square's corner away from the inside corner, leaving a concave fill -->
      <div
        class="fillet"
        style="left:{pad + f.x * unit}px; top:{pad + f.y * unit}px; width:{WELL_R * unit}px; height:{WELL_R * unit}px; --r:{WELL_R * unit}px; --at:{f.sx < 0 ? 0 : 100}% {f.sy < 0 ? 0 : 100}%"
      ></div>
    {/each}
    <!-- status LEDs (Caps Lock etc.): definition's non-VIA "periph.indicators", [x, y] centers in key units -->
    {#each def.periph?.indicators ?? [] as [x, y]}
      <span class="ind" style="left:{pad + x * unit}px; top:{pad + y * unit}px"></span>
    {/each}
    {#each def.keys.list as k, i (k.row + "," + k.col + "," + k.x)}
      {@const code = keymap?.[k.row]?.[k.col] ?? 0}
      {@const d = describe(code, def)}
      {@const key = `${k.row},${k.col}`}
      <button
        class="cap absolute {tone(k.color)}"
        data-key={key}
        class:enc={k.encoder !== null}
        class:sel={selected === key}
        class:trans={code === 1}
        class:word-key={fx === "word" && i === wordKey}
        style="left:{pad + k.x * unit}px; top:{pad + k.y * unit}px; width:{k.w * unit}px; height:{k.h * unit}px; --wr:{wellRadius.get(`${k.row},${k.col},${k.x}`) ?? "50%"}; {capLight(k, i)}"
        title={k.encoder !== null ? knobTitle(k.encoder, d) : d.long}
        aria-label={k.encoder !== null ? knobTitle(k.encoder, d) : d.long}
        onclick={() => {
          if (duel) duelHit(k);
          onselect?.(key, k);
        }}
        onkeydown={(e) => {
          if (duel && e.key === "Enter" && e.repeat) e.preventDefault();
        }}
        onpointerdown={duel ? undefined : () => hit(k)}
        onwheel={k.encoder !== null && !mini ? (ev) => wheel(ev, k.encoder) : undefined}
      >
        {#if fx && k.encoder === null}<span class="led"></span>{/if}
        {#if k.encoder !== null}
          <span class="well"></span>
          <span class="knob" style="--turn:{turn[k.encoder] ?? 0}deg">
            <span class="knurl"></span>
            <span class="top"></span>
            {#if !mini}<span class="klbl" style="font-size:{fitKnob(d.short)}px">{d.short}</span>{/if}
          </span>
          {#if flash?.enc === k.encoder}<span class="flash">{flash.text}</span>{/if}
        {:else}
          <span class="skirt">
            <span class="top">
              {#if mini}{:else if code === 0x2c}<span class="space-bar"></span><!-- space: a centered bar, like the printed legend -->
              {:else}<span class="lbl" style="font-size:{fit(d.short, k)}px">{d.short}</span>{/if}
            </span>
          </span>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .stage { perspective: 1600px; padding-bottom: 6px; }
  .tilted .case { transform: rotateX(16deg); transform-origin: 50% 60%; }

  /* ---- case: white anodized shell with a visible front edge ---- */
  .case {
    /* white finish; .black below swaps the palette */
    --case-a: #fbfbf9; --case-b: #ecece8; --case-c: #d8d8d3; --case-d: #cfcfca; --case-e: #b9b9b4;
    --hl: #fff; --face-hi: #fff;
    --plate: #dcdcd7;
    border-radius: calc(var(--pad) * 0.9);
    background: linear-gradient(180deg, var(--case-a) 0%, var(--case-b) 100%);
    box-shadow:
      inset 0 1px 0 var(--hl),
      inset 0 -1px 0 var(--case-c),
      0 calc(var(--u) * 0.16) 0 var(--case-d),
      0 calc(var(--u) * 0.16 + 1px) 0 var(--case-e),
      0 calc(var(--u) * 0.5) calc(var(--u) * 0.9) calc(var(--u) * -0.2) rgb(0 0 0 / 0.55);
    transition: transform 400ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .case.mini { box-shadow: inset 0 1px 0 var(--hl), 0 2px 0 var(--case-d), 0 6px 10px -4px rgb(0 0 0 / 0.5); border-radius: 4px; }
  /* each well overhangs its key a little so neighbours overlap (no notches where corners meet)
     and a cluster's outer corners come out rounded */
  .plate {
    position: absolute;
    margin: calc(var(--u) * -0.05); /* WELL_E */
    padding: calc(var(--u) * 0.05);
    box-sizing: content-box;
    border-radius: calc(var(--u) * 0.12);
    background: var(--plate);
  }
  .fillet {
    position: absolute;
    background: radial-gradient(circle var(--r) at var(--at), transparent calc(var(--r) - 0.5px), var(--plate) var(--r));
  }

  .ind {
    position: absolute;
    width: max(2px, calc(var(--u) * 0.08));
    aspect-ratio: 1;
    translate: -50% -50%;
    border-radius: 50%;
    background: #fdfdfb;
    box-shadow: 0 0 0 1px #c9c9c3, 0 0 calc(var(--u) * 0.08) rgb(255 255 255 / 0.9);
  }

  /* ---- keycaps ---- */
  .cap { padding: calc(var(--u) * 0.06); outline: none; --cap: #fbfbf9; --side: #e2e2dd; --edge: #c8c8c2; --ink: #45453f; }
  .mod { --cap: #f3f3ef; --side: #dcdcd6; --edge: #c2c2bb; }
  .accent { --cap: #ebebe6; --side: #d4d4cd; --edge: #babab2; }
  .mini .cap { padding: 1px; }

  .skirt {
    position: relative;
    display: block;
    height: 100%;
    width: 100%;
    border-radius: calc(var(--u) * 0.14);
    background: linear-gradient(180deg, var(--side) 0%, color-mix(in oklch, var(--side) 80%, #a9a9a2) 100%);
    box-shadow: 0 1px 0 var(--edge), 0 2px 3px rgb(0 0 0 / 0.18);
    transition: transform 120ms ease, box-shadow 200ms ease;
  }
  .skirt > .top {
    position: absolute;
    top: calc(var(--u) * 0.05);
    left: calc(var(--u) * 0.11);
    right: calc(var(--u) * 0.11);
    bottom: calc(var(--u) * 0.2);
    display: flex;
    align-items: flex-start;
    justify-content: flex-start;
    padding: calc(var(--u) * 0.07) calc(var(--u) * 0.07);
    border-radius: calc(var(--u) * 0.1);
    background: radial-gradient(120% 90% at 50% 15%, var(--face-hi) 0%, var(--cap) 55%, color-mix(in oklch, var(--cap) 92%, #b5b5ad) 100%);
    box-shadow: inset 0 1px 0 var(--hl), 0 1px 1px rgb(0 0 0 / 0.08);
  }
  .mini .skirt { border-radius: 2px; }
  .mini .skirt > .top { top: 0; left: 1px; right: 1px; bottom: 1px; border-radius: 1.5px; padding: 0; }

  /* legends are translucent (shine-through): frosted grey when off, the LED color when lit */
  .skirt > .top, .klbl { --legend: rgb(62 62 56 / 0.62); }
  .lbl, .klbl {
    color: var(--legend);
    text-shadow: 0 1px 0 rgb(255 255 255 / 0.7);
  }
  .lbl { font-weight: 650; line-height: 1; white-space: nowrap; letter-spacing: -0.01em; }
  .trans .lbl { opacity: 0.4; }
  .space-bar {
    position: absolute;
    left: 50%;
    top: 50%;
    width: min(36%, calc(var(--u) * 1.6));
    height: max(1.5px, calc(var(--u) * 0.045));
    translate: -50% -50%;
    border-radius: 99px;
    background: var(--legend);
    box-shadow: 0 1px 0 rgb(255 255 255 / 0.7);
  }
  .black .space-bar { box-shadow: 0 -1px 0 rgb(0 0 0 / 0.5); }
  .sel .space-bar { background: color-mix(in oklch, var(--brand) 80%, #000); }

  .cap:hover .skirt { transform: translateY(-1.5px); box-shadow: 0 1px 0 var(--edge), 0 5px 8px rgb(0 0 0 / 0.2); }
  .cap:active .skirt { transform: translateY(1px); box-shadow: 0 0 0 var(--edge), 0 1px 1px rgb(0 0 0 / 0.2); }
  .sel .skirt, .sel .knob {
    box-shadow: 0 0 0 2px var(--brand), 0 0 0 5px color-mix(in oklch, var(--brand) 35%, transparent), 0 4px 10px rgb(0 0 0 / 0.25);
  }
  .sel .lbl, .sel .klbl { color: color-mix(in oklch, var(--brand) 80%, #000); }

  /* ---- rotary knob (volume) ---- */
  .knob {
    position: absolute;
    left: 50%;
    top: 50%;
    width: calc(var(--u) * 0.96);
    aspect-ratio: 1;
    translate: -50% -72%;
    border-radius: 50%;
    /* stacked offsets = the knob's cylindrical side; taller than the keycaps */
    box-shadow:
      0 calc(var(--u) * 0.06) 0 #dcdcd6,
      0 calc(var(--u) * 0.12) 0 #d0d0ca,
      0 calc(var(--u) * 0.18) 0 #c4c4bd,
      0 calc(var(--u) * 0.23) 0 #aaaaa3,
      0 calc(var(--u) * 0.27) calc(var(--u) * 0.2) rgb(0 0 0 / 0.35);
    transition: box-shadow 200ms ease, translate 120ms ease;
  }
  .well {
    position: absolute;
    left: 50%;
    top: 50%;
    width: calc(var(--u) * 1.02);
    aspect-ratio: 1;
    translate: -50% -50%;
    border-radius: 50%;
    background: #cfcfc9;
    box-shadow: inset 0 2px 4px rgb(0 0 0 / 0.25);
  }
  .flash {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 4px);
    translate: -50% 0;
    padding: 2px 7px;
    border-radius: 6px;
    background: var(--popover, #111);
    color: var(--popover-foreground, #fff);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    pointer-events: none;
    z-index: 2;
    animation: pop 160ms ease-out;
  }
  @keyframes pop { from { opacity: 0; translate: -50% 4px; } }
  .knurl {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: radial-gradient(circle, #f4f4f0 60%, #e2e2dc 100%); /* smooth, unknurled */
    rotate: var(--turn);
    transition: rotate 450ms cubic-bezier(0.3, 1.4, 0.4, 1);
  }
  .knob > .top {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 30%, #ffffff 0%, #f4f4f0 60%, #f0f0eb 100%);
    rotate: var(--turn);
    transition: rotate 450ms cubic-bezier(0.3, 1.4, 0.4, 1);
  }
  .klbl {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding-top: calc(var(--u) * 0.22);
    line-height: 1;
    font-weight: 600;
    pointer-events: none;
  }
  .cap.enc:active .knob { translate: -50% -66%; }
  .mini .knob { box-shadow: 0 1px 0 #b4b4ad; translate: -50% -50%; }
  .mini .well { display: none; }

  /* ---- RGB preview ----
     --H hue (deg), --L lightness (HSV saturation mapped to HSL), --a brightness, --dur effect period.
     Per key: --t animates 0..1 (rainbow), --k is the key's light level (breathing / rain / reactive),
     --hs is a hue shift used by reactive flashes. --t/--k/--hs are registered in app.css. */
  .lit .cap {
    --hue: calc(var(--H) + var(--hs) + (var(--t) + var(--ph, 0)) * 360);
    --lc: hsl(var(--hue) 100% var(--L));
    --on: calc(var(--a) * var(--k));
  }
  .fx-duel .cap { --lc: var(--duel-color); }
  .fx-word .cap:not(.word-key) { --k: 0; }
  /* the glow fills the key's plate well (key + WELL_E) and is clipped there, so it never spills
     onto the raised case between clusters or around the edge */
  .led {
    position: absolute;
    inset: calc(var(--u) * -0.05);
    border-radius: var(--wr);
    background: var(--lc);
    filter: blur(calc(var(--u) * 0.14));
    clip-path: inset(0 round var(--wr));
    opacity: var(--on);
    pointer-events: none;
  }
  /* the knob has no LED and stays plain in the lighting view */
  .lit .klbl { display: none; }
  .lit { --plate: #c9c9c3; }
  /* shine-through: translucent skirt and a faint wash on the top face */
  .lit .skirt {
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--side) calc(100% - var(--on) * 45%), var(--lc)) 0%,
      color-mix(in oklch, var(--side) calc(100% - var(--on) * 75%), var(--lc)) 100%);
  }
  .lit .skirt > .top {
    background: radial-gradient(120% 90% at 50% 15%, var(--face-hi) 0%,
      color-mix(in oklch, var(--cap) calc(100% - var(--on) * 5%), var(--lc)) 60%,
      color-mix(in oklch, var(--cap) calc(100% - var(--on) * 14%), var(--lc)) 100%);
  }
  /* solid (opaque) keycaps: the sides and face stay unlit; light only escapes between the keys
     (the .led underglow) and through the legends */
  .solid.lit .skirt { background: linear-gradient(180deg, var(--side) 0%, color-mix(in oklch, var(--side) 80%, #a9a9a2) 100%); }
  .solid.lit .skirt > .top { background: radial-gradient(120% 90% at 50% 15%, var(--face-hi) 0%, var(--cap) 55%, color-mix(in oklch, var(--cap) 92%, #b5b5ad) 100%); }
  .lit .space-bar {
    --lg: var(--duel-color, hsl(var(--hue) var(--S) 50%));
    background: color-mix(in oklch, var(--lg) calc(var(--on) * 92%), var(--legend));
    box-shadow:
      0 0 calc(var(--u) * 0.05) color-mix(in oklch, var(--lc) calc(var(--on) * 90%), transparent),
      0 0 calc(var(--u) * 0.16) color-mix(in oklch, var(--lc) calc(var(--on) * 60%), transparent);
  }
  /* light passing through the legend: colored text + a soft bloom around it */
  .lit .lbl, .lit .klbl {
    /* same hue+saturation at mid lightness, so pale / white light stays readable on white caps */
    --lg: var(--duel-color, hsl(var(--hue) var(--S) 50%));
    color: color-mix(in oklch, var(--lg) calc(var(--on) * 92%), var(--legend));
    text-shadow:
      0 0 calc(var(--u) * 0.05) color-mix(in oklch, var(--lc) calc(var(--on) * 90%), transparent),
      0 0 calc(var(--u) * 0.16) color-mix(in oklch, var(--lc) calc(var(--on) * 60%), transparent);
  }
  .case.lit {
    box-shadow:
      inset 0 1px 0 var(--hl),
      inset 0 -1px 0 var(--case-c),
      0 calc(var(--u) * 0.16) 0 var(--case-d),
      0 calc(var(--u) * 0.16 + 1px) 0 var(--case-e),
      0 calc(var(--u) * 0.5) calc(var(--u) * 0.9) calc(var(--u) * -0.2) rgb(0 0 0 / 0.55),
      0 0 calc(var(--u) * 1.4) calc(var(--u) * -0.3) color-mix(in oklch, hsl(var(--H) 100% var(--L)) calc(var(--a) * 50%), transparent);
  }
  .case.lit.rainbow, .case.lit.fx-heat, .case.lit.fx-duel, .case.lit.fx-word { box-shadow: inset 0 1px 0 var(--hl), inset 0 -1px 0 var(--case-c), 0 calc(var(--u) * 0.16) 0 var(--case-d), 0 calc(var(--u) * 0.16 + 1px) 0 var(--case-e), 0 calc(var(--u) * 0.5) calc(var(--u) * 0.9) calc(var(--u) * -0.2) rgb(0 0 0 / 0.55); }

  .rainbow .cap { animation: spin var(--dur) linear infinite; }
  .fx-breathe .cap { animation: breathe var(--dur) ease-in-out infinite; }
  .fx-rain .cap { --hs: var(--rh, 0); animation: drop var(--dur) ease-out infinite; animation-delay: calc(var(--rd, 0) * -1s); }
  .fx-simple .cap, .fx-cross .cap, .fx-splash .cap { --k: 0.06; }
  .fx-reactive .cap { --k: 0.55; }
  .fx-heat .cap { --k: 0; --lc: hsl(calc(70 - var(--k) * 70) 100% 50%); }
  @keyframes spin { from { --t: 0; } to { --t: 1; } }
  @keyframes breathe { 0%, 100% { --k: 0.08; } 50% { --k: 1; } }
  @keyframes drop { 0% { --k: 0.12; } 6% { --k: 1; } 40% { --k: 0.12; } 100% { --k: 0.12; } }

  /* ---- black finish ---- */
  .case.black {
    --case-a: #2a2a2d; --case-b: #1d1d20; --case-c: #111113; --case-d: #0e0e10; --case-e: #050506;
    --hl: rgb(255 255 255 / 0.1); --face-hi: #3a3a3e;
    --plate: #111113;
  }
  .case.black.lit { --plate: #0b0b0c; }
  .black .cap { --cap: #2b2b2e; --side: #1f1f22; --edge: #0b0b0c; --ink: #d9d9d4; }
  .black .mod { --cap: #27272a; --side: #1b1b1e; --edge: #09090a; }
  .black .accent { --cap: #232326; --side: #18181a; --edge: #070708; }
  .black .skirt > .top, .black .klbl { --legend: rgb(226 226 220 / 0.62); }
  .black .lbl, .black .klbl { text-shadow: 0 -1px 0 rgb(0 0 0 / 0.5); }
  .black .ind { box-shadow: 0 0 0 1px #3a3a3e, 0 0 calc(var(--u) * 0.08) rgb(255 255 255 / 0.9); }
  .black .well { background: #0e0e10; }
  .black .knob {
    box-shadow:
      0 calc(var(--u) * 0.06) 0 #212124,
      0 calc(var(--u) * 0.12) 0 #1a1a1d,
      0 calc(var(--u) * 0.18) 0 #141416,
      0 calc(var(--u) * 0.23) 0 #0a0a0b,
      0 calc(var(--u) * 0.27) calc(var(--u) * 0.2) rgb(0 0 0 / 0.5);
  }
  .black .knurl { background: radial-gradient(circle, #2c2c2f 60%, #1f1f22 100%); }
  .black .knob > .top { background: radial-gradient(circle at 38% 30%, #3c3c40 0%, #2c2c2f 60%, #28282b 100%); }
  .black.mini .knob { box-shadow: 0 1px 0 #0a0a0b; }
</style>
