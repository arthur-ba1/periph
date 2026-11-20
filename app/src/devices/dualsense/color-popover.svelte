<script>
  // Custom lightbar colour: a round rainbow swatch that opens a picker with a saturation /
  // brightness field, a hue strip and a HEX field. Dragging previews live (`oninput`); the
  // colour is sent once on release (`onchange`).
  import { Popover } from "bits-ui";
  import { toHex, fromHex } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  /** value: [r, g, b]; selected: the current colour is not a preset (ring the swatch) */
  let { value, selected = false, oninput, onchange } = $props();

  let h = $state(0), s = $state(1), v = $state(1);
  let hex = $state("");
  const id = $props.id();

  const clamp = (n) => Math.max(0, Math.min(1, n));
  function read([r, g, b]) {
    [r, g, b] = [r / 255, g / 255, b / 255];
    const max = Math.max(r, g, b), c = max - Math.min(r, g, b);
    v = max;
    s = max ? c / max : 0;
    if (c) h = ((max === r ? (g - b) / c : max === g ? (b - r) / c + 2 : (r - g) / c + 4) * 60 + 360) % 360;
  }
  function rgbOf(hh, ss, vv) {
    const f = (n) => {
      const k = (n + hh / 60) % 6;
      return Math.round((vv - vv * ss * Math.max(0, Math.min(k, 4 - k, 1))) * 255);
    };
    return [f(5), f(3), f(1)];
  }
  let rgb = $derived(rgbOf(h, s, v));
  let pure = $derived(toHex(rgbOf(h, 1, 1)));

  function emit(commit) {
    hex = toHex(rgb).toUpperCase();
    (commit ? onchange : oninput)?.(rgb);
  }

  /** Pointer drag helper: calls `at(x01, y01)` while the pointer is held, commits on release. */
  function drag(at) {
    return {
      onpointerdown(e) {
        if (e.button !== 0) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        move(e);
      },
      onpointermove(e) {
        if (e.currentTarget.hasPointerCapture(e.pointerId)) move(e);
      },
      onpointerup(e) {
        if (e.currentTarget.hasPointerCapture(e.pointerId)) emit(true);
      },
    };
    function move(e) {
      const r = e.currentTarget.getBoundingClientRect();
      at(clamp((e.clientX - r.left) / r.width), clamp((e.clientY - r.top) / r.height));
      emit(false);
    }
  }
  const field = drag((x, y) => ((s = x), (v = 1 - y)));
  const strip = drag((x) => (h = x * 359.9));

  function keys(e, kind) {
    const step = e.shiftKey ? 0.1 : 0.02;
    const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] }[e.key];
    if (!d) return;
    e.preventDefault();
    if (kind === "field") ((s = clamp(s + d[0])), (v = clamp(v + d[1])));
    else h = (h + d[0] * 360 + d[1] * 360 + 360) % 360;
    emit(true);
  }

  function editHex(e) {
    hex = e.currentTarget.value;
    const m = hex.trim().replace(/^#/, "");
    if (!/^[0-9a-f]{6}$/i.test(m)) return;
    read(fromHex("#" + m));
    emit(true);
  }

  function opened(open) {
    if (!open) return;
    read(value);
    hex = toHex(value).toUpperCase();
  }
</script>

<Popover.Root onOpenChange={opened}>
  <Popover.Trigger>
    {#snippet child({ props })}
      <button
        {...props}
        type="button"
        class="custom grid size-7 place-items-center rounded-full p-[3px] transition-transform hover:scale-110 {selected ? 'ring-foreground ring-2 ring-offset-2 ring-offset-[var(--color-card)]' : ''}"
        aria-label={t("dualsense.lights.custom")}
        title={t("dualsense.lights.custom")}
      >
        <span class="bg-card grid size-full place-items-center rounded-full">
          {#if selected}
            <span class="size-3 rounded-full" style:background={toHex(value)}></span>
          {:else}
            <svg viewBox="0 0 12 12" class="size-3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M6 2.5v7M2.5 6h7" /></svg>
          {/if}
        </span>
      </button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content side="bottom" align="end" sideOffset={8} aria-label={t("dualsense.lights.custom")}
      class="bg-popover text-popover-foreground ring-foreground/10 z-50 grid w-60 gap-3 rounded-xl p-3 shadow-xl ring-1">
      <!-- saturation (x) / brightness (y) -->
      <div
        role="slider"
        tabindex="0"
        aria-label={t("dualsense.lights.pickerField")}
        aria-valuenow={Math.round(s * 100)}
        class="sv relative h-36 cursor-crosshair touch-none rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-ring)]"
        style:--pure={pure}
        {...field}
        onkeydown={(e) => keys(e, "field")}
      >
        <span class="marker" style:left="{s * 100}%" style:top="{(1 - v) * 100}%" style:background={toHex(rgb)}></span>
      </div>
      <!-- hue -->
      <div
        role="slider"
        tabindex="0"
        aria-label={t("dualsense.lights.pickerHue")}
        aria-valuemin={0}
        aria-valuemax={360}
        aria-valuenow={Math.round(h)}
        class="hue relative h-3 cursor-pointer touch-none rounded-full outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-ring)]"
        {...strip}
        onkeydown={(e) => keys(e, "hue")}
      >
        <span class="marker top-1/2" style:left="{(h / 360) * 100}%" style:background={pure}></span>
      </div>
      <div class="flex items-center gap-2">
        <span class="size-8 shrink-0 rounded-lg border border-white/15 shadow-[0_0_14px_var(--c)]" style:background={toHex(rgb)} style:--c={toHex(rgb)}></span>
        <label for={id} class="text-muted-foreground text-[11px] font-medium">HEX</label>
        <input
          {id}
          type="text"
          value={hex}
          oninput={editHex}
          onblur={() => (hex = toHex(rgb).toUpperCase())}
          maxlength={7}
          spellcheck={false}
          autocomplete="off"
          class="border-input bg-background/50 focus-visible:ring-ring h-8 w-full min-w-0 rounded-md border px-2 font-mono text-xs uppercase outline-none focus-visible:ring-2"
        />
      </div>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  .custom { background: conic-gradient(#ff3b3b, #ffd23b, #3bff6a, #3bd8ff, #3b5bff, #d43bff, #ff3b3b); }
  .sv { background: linear-gradient(to top, black, transparent), linear-gradient(to right, white, var(--pure)); }
  .hue { background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00); }
  .marker {
    position: absolute;
    width: 16px;
    height: 16px;
    transform: translate(-50%, -50%);
    border: 2.5px solid white;
    border-radius: 9999px;
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.5), 0 2px 6px rgb(0 0 0 / 0.45);
    pointer-events: none;
  }
</style>
