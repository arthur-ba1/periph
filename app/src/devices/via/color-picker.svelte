<script>
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import { Popover } from "bits-ui";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { t } from "$lib/i18n.svelte.js";

  let { label, value = $bindable() } = $props();
  let hex = $state("");
  const hexId = $props.id();
  const clamp = (n) => Math.max(0, Math.min(1, n));
  let hue = $state(0);
  let saturation = $state(0);
  let brightness = $state(1);
  let angle = $derived(hue * Math.PI / 180);
  let markerX = $derived(50 + Math.sin(angle) * saturation * 50);
  let markerY = $derived(50 - Math.cos(angle) * saturation * 50);

  function readColor(color) {
    const [r, g, b] = [1, 3, 5].map((offset) => Number.parseInt(color.slice(offset, offset + 2), 16) / 255);
    brightness = Math.max(r, g, b);
    const chroma = brightness - Math.min(r, g, b);
    if (brightness > 0) saturation = chroma / brightness;
    if (chroma === 0) return;
    const sector = brightness === r ? (g - b) / chroma
      : brightness === g ? (b - r) / chroma + 2 : (r - g) / chroma + 4;
    hue = (sector * 60 + 360) % 360;
  }

  function colorAt(level) {
    const sector = hue / 60;
    const chroma = level * saturation;
    const second = chroma * (1 - Math.abs(sector % 2 - 1));
    const rgb = [[chroma, second, 0], [second, chroma, 0], [0, chroma, second],
      [0, second, chroma], [second, 0, chroma], [chroma, 0, second]][Math.floor(sector)];
    return "#" + rgb.map((channel) => Math.round((channel + level - chroma) * 255).toString(16).padStart(2, "0")).join("");
  }

  function applyColor() {
    value = colorAt(brightness);
    hex = value.toUpperCase();
  }

  function openPicker(open) {
    if (!open) return;
    readColor(value);
    hex = value.toUpperCase();
  }

  function editHex(event) {
    hex = event.currentTarget.value;
    if (!/^#?[0-9a-f]{6}$/i.test(hex)) return;
    value = "#" + hex.replace(/^#/, "").toLowerCase();
    readColor(value);
  }

  function pickFromPointer(event) {
    const { left, top, width } = event.currentTarget.getBoundingClientRect();
    const radius = width / 2;
    const x = event.clientX - left - radius;
    const y = event.clientY - top - radius;
    saturation = clamp(Math.hypot(x, y) / radius);
    if (saturation > 0) hue = (Math.atan2(x, -y) * 180 / Math.PI + 360) % 360;
    applyColor();
  }

  function startDrag(event) {
    if (!event.isPrimary || event.button !== 0) return;
    event.currentTarget.focus();
    event.currentTarget.setPointerCapture(event.pointerId);
    pickFromPointer(event);
  }

  function moveDrag(event) {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) pickFromPointer(event);
  }

  function adjustWheel(event) {
    switch (event.key) {
      case "ArrowLeft": hue = (hue + 355) % 360; break;
      case "ArrowRight": hue = (hue + 5) % 360; break;
      case "ArrowDown": saturation = clamp(saturation - 0.05); break;
      case "ArrowUp": saturation = clamp(saturation + 0.05); break;
      default: return;
    }
    event.preventDefault();
    applyColor();
  }
</script>

<Popover.Root onOpenChange={openPicker}>
  <Popover.Trigger>
    {#snippet child({ props })}
      <Button {...props} variant="outline" class="h-auto min-w-0 justify-start gap-2.5 px-3 py-2 text-left">
        <span aria-hidden="true" class="border-border size-7 shrink-0 rounded-md border" style:background={value}></span>
        <span class="grid min-w-0 flex-1 gap-0.5 whitespace-normal">
          <span class="leading-tight">{label}</span>
          <span class="text-muted-foreground font-mono text-xs">{value.toUpperCase()}</span>
        </span>
        <ChevronDown aria-hidden="true" class="text-muted-foreground size-4" />
      </Button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content align="start" sideOffset={6} aria-label={label} data-color-picker
      class="bg-popover text-popover-foreground ring-foreground/10 z-50 grid w-64 max-w-[calc(100vw-1rem)] gap-4 rounded-lg p-4 shadow-md ring-1">
      <p id={hexId + "-wheel-help"} class="sr-only">{t("via.lighting.duel.wheelHelp")}</p>
      <div role="group" tabindex="0" aria-label={t("via.lighting.duel.wheel")}
        aria-describedby={hexId + "-wheel-help"}
        class="wheel ring-border focus-visible:ring-ring mx-auto aspect-square w-full max-w-48 touch-none rounded-full ring-1 focus-visible:ring-3 focus-visible:outline-none"
        onpointerdown={startDrag} onpointermove={moveDrag} onkeydown={adjustWheel}>
        <span aria-hidden="true" class="marker" style:left={markerX + "%"} style:top={markerY + "%"} style:background={value}></span>
      </div>
      <div class="brightness grid gap-2" style:--full-color={colorAt(1)}>
        <div class="flex items-center justify-between gap-2 text-xs">
          <span>{t("via.lighting.labels.Brightness")}</span>
          <span class="text-muted-foreground font-mono">{Math.round(brightness * 100)}%</span>
        </div>
        <Slider type="single" min={0} max={100} step={1} value={Math.round(brightness * 100)}
          thumbLabel={t("via.lighting.labels.Brightness")}
          onValueChange={(amount) => { brightness = amount / 100; applyColor(); }} />
      </div>
      <div class="flex items-center gap-2">
        <label for={hexId} class="text-muted-foreground text-xs font-medium">HEX</label>
        <Input id={hexId} type="text" value={hex} oninput={editHex}
          onblur={() => (hex = value.toUpperCase())} maxlength={7} autocomplete="off" spellcheck={false} class="font-mono" />
      </div>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  .wheel {
    position: relative;
    cursor: crosshair;
    background:
      radial-gradient(circle closest-side, white, transparent),
      conic-gradient(in srgb, red, yellow, lime, cyan, blue, magenta, red);
  }
  .marker {
    position: absolute;
    width: 15px;
    height: 15px;
    transform: translate(-50%, -50%);
    border: 2px solid white;
    border-radius: 50%;
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.65), 0 1px 4px rgb(0 0 0 / 0.5);
    pointer-events: none;
  }
  .brightness :global([data-slot="slider-track"]) { background: linear-gradient(90deg, black, var(--full-color)); }
  .brightness :global([data-slot="slider-range"]) { background: transparent; }
</style>
