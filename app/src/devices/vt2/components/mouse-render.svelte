<script module>
  // VT2 Max renders + per-button highlight overlays, extracted from the official A HUB
  // resources (Image/Keys/VT2). They are not in the repository (Rapoo's copyright): extract them
  // from your own A HUB install, see ../assets/NOTICE.md. Without them the UI still works and
  // shows a plain placeholder instead of the render.
  const bgs = import.meta.glob("../assets/background_*.png", { eager: true, import: "default" });
  const raw = import.meta.glob("../assets/*.svg", { eager: true, query: "?raw", import: "default" });
  const device = Object.values(import.meta.glob("../assets/device.json", { eager: true, import: "default" }))[0];

  const FILES = ["Mouse left", "Mouse middle", "Mouse right", "DPI+", "DPI-", "Mouse forward", "Mouse back", "Roller forward", "Roller back", "DIY"];
  const TOP_CENTER = 290;

  // Recolor to the design-system accent and find the leader-line dot (label anchor).
  export const ZONES = FILES.map((name, index) => {
    let svg = raw[`../assets/${name}.svg`] ?? "";
    svg = svg.replace(/(stop-color|fill)="#205EFF"/g, 'style="$1:var(--brand)"');
    // leader path: solid fill, starts with a small circle "M x y C x1 y1 x2 y2 cx ..."
    const m = svg.match(/<path d="M([\d.]+) ([\d.]+)C[\d.]+ [\d.]+ [\d.]+ [\d.]+ ([\d.]+) [\d.]+[^"]*" style="fill/);
    const dot = m ? { x: +m[3], y: +m[2] } : null;
    return { index, name, svg, dot, left: dot ? dot.x < TOP_CENTER : false };
  });

  export const HAS_ART = Object.keys(bgs).length > 0;
  export const COLORS = device?.ProductColorType.map((c) => ({ key: c.key, name: c.name, hex: c.rectColor })) ?? [];

  // Source art is 941x456: top view on the left (mouse spans x 190..390), underside on the right
  // (x 549..749). We show one view at a time; both crops are centered on the mouse so flipping
  // turns it in place instead of shifting it sideways.
  export const CROP = { x: 80, w: 420, h: 410 };
  const UNDER_X = 439.5;
  export const DIY = ZONES.findIndex((z) => z.name === "DIY");
  const TOP_ZONES = ZONES.filter((z) => z.name !== "DIY");
  const UNDER_ZONES = ZONES.filter((z) => z.name === "DIY");
  export const bgFor = (color) => bgs[`../assets/background_${color}.png`];

  // The A HUB art fades the lower half of the mouse to transparent (alpha ramps 255 -> 0 between
  // y ~255 and ~388), which reads as the mouse being cut off. The colors under the fade are intact,
  // so undo it: divide each row's alpha by the row's fade factor (its max alpha). The last rows,
  // where the fade hits ~0, are rebuilt from the full-length thumbnail's silhouette.
  const thumbs = import.meta.glob("../assets/thumb_*.png", { eager: true, import: "default" });
  const solid = new Map();
  const loadImg = (src) => new Promise((ok, err) => { const i = new Image(); i.onload = () => ok(i); i.onerror = err; i.src = src; });
  const pixels = (img) => {
    const c = document.createElement("canvas");
    c.width = img.width;
    c.height = img.height;
    const g = c.getContext("2d", { willReadFrequently: true });
    g.drawImage(img, 0, 0);
    return { c, g, data: g.getImageData(0, 0, c.width, c.height) };
  };
  async function unfade(color) {
    const [bg, th] = await Promise.all([loadImg(bgFor(color)), loadImg(thumbs[`../assets/thumb_${color}.png`])]);
    const { c, g, data } = pixels(bg);
    const t = pixels(th).data;
    const W = c.width, H = c.height, d = data.data;
    const MIN = 14; // below this fade factor the colors are too quantized to trust
    for (const [hx0, hx1, mirror] of [[0, 470, false], [470, W, true]]) {
      const mask = new Array(H).fill(0);
      let x0 = W, x1 = -1;
      for (let y = 0; y < H; y++)
        for (let x = hx0; x < hx1; x++) {
          const a = d[(y * W + x) * 4 + 3];
          if (a > mask[y]) mask[y] = a;
          if (a > 128) (x0 = Math.min(x0, x)), (x1 = Math.max(x1, x));
        }
      let yc = H;
      for (let y = 200; y < H; y++) { // fade only covers the lower part; top-edge antialiasing stays
        if (mask[y] === 255) continue;
        if (mask[y] < MIN) { yc = y; break; }
        const k = 255 / mask[y];
        for (let x = hx0; x < hx1; x++) { const i = (y * W + x) * 4 + 3; d[i] = Math.min(255, Math.round(d[i] * k)); }
      }
      // tail: thumbnail silhouette scaled to the mouse's width, colors carried down from row yc-1
      const s = (x1 - x0 + 1) / th.width;
      for (let y = yc; y < H; y++) {
        const ty = Math.floor(y / s);
        for (let x = hx0; x < hx1; x++) {
          const i = (y * W + x) * 4;
          let tx = Math.floor((x - x0) / s);
          if (mirror) tx = th.width - 1 - tx;
          const ta = ty < th.height && tx >= 0 && tx < th.width ? t.data[(ty * th.width + tx) * 4 + 3] : 0;
          if (!ta) { d[i + 3] = 0; continue; }
          let sx = x, mid = (x0 + x1) >> 1;
          while (d[((yc - 1) * W + sx) * 4 + 3] < 128 && sx !== mid) sx += sx < mid ? 1 : -1;
          const j = ((yc - 1) * W + sx) * 4;
          d[i] = d[j]; d[i + 1] = d[j + 1]; d[i + 2] = d[j + 2]; d[i + 3] = ta;
        }
      }
    }
    g.putImageData(data, 0, 0);
    return URL.createObjectURL(await new Promise((ok) => c.toBlob(ok, "image/png")));
  }
  export const solidBgFor = (color) => {
    if (!solid.has(color)) solid.set(color, unfade(color).catch(() => bgFor(color)));
    return solid.get(color);
  };
</script>

<script>
  import { fade } from "svelte/transition";
  let { color = 4, selected = null, labels = {}, onselect, interactive = true } = $props();
  let hovered = $state(null);
  $effect(() => {
    if (!interactive) hovered = null;
  });

  // Selecting the DIY button flips the mouse over. The flip squeezes the view to 0 width,
  // swaps the crop at the midpoint and opens it again, like turning the mouse in your hand.
  let wantUnder = $derived(interactive && selected === DIY);
  let under = $state(false);
  let squeeze = $state(false);
  let flipTimer;
  $effect(() => {
    const target = wantUnder;
    if (target === under) return;
    clearTimeout(flipTimer);
    squeeze = true;
    flipTimer = setTimeout(() => {
      under = target;
      hovered = null;
      squeeze = false;
    }, 170);
    return () => clearTimeout(flipTimer);
  });
  let cropX = $derived(under ? UNDER_X : CROP.x);
  let zones = $derived(under ? UNDER_ZONES : TOP_ZONES);

  let srcs = $state({});
  $effect(() => {
    const c = color;
    if (HAS_ART && bgFor(c) && !srcs[c]) solidBgFor(c).then((u) => (srcs[c] = u));
  });
</script>

<!-- outer box = cropped top view; inner box = full-size art shifted so the crop lines up -->
<div class="relative w-full select-none" style="aspect-ratio:{CROP.w}/{CROP.h}" class:passive={!interactive}>
{#if !HAS_ART}
  <div class="text-muted-foreground/40 absolute inset-0 grid place-items-center">
    <svg viewBox="0 0 24 24" class="h-3/4" fill="none" stroke="currentColor" stroke-width="0.6"><rect x="6" y="2.5" width="12" height="19" rx="6" /><path d="M12 2.5v6M12 5v1.6" /></svg>
  </div>
{/if}
<div class="flip absolute inset-0" class:squeeze>
<div class="absolute top-0" style="left:{(-cropX / CROP.w) * 100}%; width:{(941 / CROP.w) * 100}%; aspect-ratio:941/456">
  {#key srcs[color]}
    {#if srcs[color]}
    <img src={srcs[color]} alt="Rapoo VT2 Max" class="absolute inset-0 size-full" style="clip-path: inset(0 {100 - ((cropX + CROP.w) / 941) * 100}% 0 {(cropX / 941) * 100}%)" draggable="false" in:fade={{ duration: 250 }} />
    {/if}
  {/key}

  {#each zones as z (z.index)}
    <div
      class="zone absolute inset-0"
      class:on={selected === z.index}
      class:hov={hovered === z.index && selected !== z.index}
      onclick={() => onselect?.(z.index)}
      onpointerenter={() => (hovered = z.index)}
      onpointerleave={() => hovered === z.index && (hovered = null)}
      role="presentation"
    >
      {@html z.svg}
    </div>
  {/each}

  {#each zones as z (z.index)}
    {#if interactive && z.dot && (selected === z.index || hovered === z.index)}
      <div
        class="pointer-events-none absolute -translate-y-1/2 whitespace-nowrap"
        style="top:{(z.dot.y / 456) * 100}%; {z.left ? `right:${100 - (z.dot.x / 941) * 100}%; padding-right:14px` : `left:${(z.dot.x / 941) * 100}%; padding-left:14px`}"
        transition:fade={{ duration: 150 }}
      >
        <span class="bg-popover border-border rounded-md border px-2 py-1 text-xs font-medium shadow-lg {selected === z.index ? 'text-foreground' : 'text-muted-foreground'}">
          {labels[z.index] ?? z.name}
        </span>
      </div>
    {/if}
  {/each}
</div>
</div>
</div>

<style>
  .flip {
    transform-origin: 50% 50%;
    transition: transform 170ms cubic-bezier(0.4, 0, 0.6, 1), filter 170ms ease;
  }
  .flip.squeeze { transform: perspective(900px) rotateY(90deg) scale(0.96); filter: brightness(0.7); }
  .zone {
    pointer-events: none;
    opacity: 0;
    transition: opacity 200ms ease;
  }
  .zone :global(svg) { width: 100%; height: 100%; overflow: visible; }
  .zone :global([fill^="url"]) { pointer-events: all; cursor: pointer; }
  .zone.hov { opacity: 0.45; }
  .zone.on { opacity: 1; }
  .passive :global(*) { pointer-events: none !important; }
  .passive .zone.on { opacity: 0; }
</style>
