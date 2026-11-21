<script>
  // Persistent controller stage: one 3D view for every DualSense page. Each page leaves an
  // empty slot (page-layout.svelte, ui.slot) and this layer sits over it, so a tab switch
  // keeps the same controller on screen and only the camera moves to the new page's part.
  import { onMount } from "svelte";
  import { followLayout } from "$lib/follow-layout.js";
  import ControllerStage from "./controller-stage.svelte";
  import { ui } from "./store.svelte.js";

  /** shellEl: app grid root; mainEl: scrolling page container (from App's device layer). */
  let { shellEl, mainEl } = $props();

  let rect = $state({ left: 0, top: 0, width: 0, height: 0 });
  let shown = $derived(!!ui.slot);

  // Layout position of the slot, without the page's fly-in transform (the stage stays put
  // while the page content animates around it).
  function measure() {
    const slot = ui.slot;
    if (!slot || !shellEl) return;
    const s = shellEl.getBoundingClientRect(), r = slot.getBoundingClientRect();
    let dx = 0, dy = 0;
    for (let e = slot.parentElement; e && e !== mainEl; e = e.parentElement) {
      const tr = getComputedStyle(e).transform;
      if (tr && tr !== "none") {
        const m = new DOMMatrix(tr);
        dx += m.e;
        dy += m.f;
      }
    }
    const n = { left: r.left - s.left - dx, top: r.top - s.top - dy, width: r.width, height: r.height };
    if (n.left !== rect.left || n.top !== rect.top || n.width !== rect.width || n.height !== rect.height) rect = n;
  }

  // Follow layout on resize, scroll and page switches (a burst of frames covers the fly-in);
  // writes only on change.
  let follow = null;
  onMount(() => {
    follow = followLayout(measure, { scroller: mainEl });
    return () => {
      follow.destroy();
      follow = null;
    };
  });
  $effect(() => {
    const slot = ui.slot;
    follow?.observe([shellEl, mainEl, slot, mainEl?.firstElementChild]);
    follow?.burst();
  });
</script>

<div
  class="stage-layer absolute z-20"
  class:off={!shown}
  style="left:{rect.left}px; top:{rect.top}px; width:{rect.width}px; height:{rect.height}px"
  aria-hidden={!shown}
  onwheel={(e) => mainEl?.scrollBy({ top: e.deltaY })}
>
  <ControllerStage />
</div>

<style>
  .stage-layer { transition: opacity 250ms ease; }
  .stage-layer.off { opacity: 0; visibility: hidden; pointer-events: none; transition: none; }
</style>
