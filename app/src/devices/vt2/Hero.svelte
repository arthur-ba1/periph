<script>
  // Background layer: the big mouse render behind the pages. On the Buttons tab it flies into
  // the page's slot (ui.slot) and becomes interactive; elsewhere it sits bottom-right and shows
  // through the cards as frosted glass.
  import { onMount, untrack } from "svelte";
  import { followLayout } from "$lib/follow-layout.js";
  import MouseRender from "./components/mouse-render.svelte";
  import { vt2, prefs, ui, BUTTONS } from "./store.svelte.js";
  import { actionLabel } from "./actions.js";

  /** shellEl: app grid root; mainEl: scrolling page container; tab: active tab index */
  let { shellEl, mainEl, tab } = $props();

  const BUTTONS_TAB = 3;
  let rect = $state({ left: 0, top: 0, width: 0 });
  let docked = $derived(tab === BUTTONS_TAB && !!ui.slot && !!vt2.config);
  let animating = $state(false);
  let labels = $derived(
    Object.fromEntries(BUTTONS.map((b, i) => [i, `${b.name} · ${vt2.config ? actionLabel(vt2.config.buttons[i]) : ""}`])),
  );

  // Frosted glass without backdrop-filter (WebView2 ignores it here): a blurred copy of the
  // hero, clipped to the union of the card rectangles, sits between the hero and the cards.
  let glassClip = $state("path('M0 0Z')");
  function measureGlass(s) {
    const r = 14; // card radius (rounded-xl)
    const parts = [];
    for (const el of mainEl.querySelectorAll('[data-slot="card"]')) {
      const b = el.getBoundingClientRect();
      const x = b.left - s.left - rect.left, y = b.top - s.top - rect.top, w = b.width, h = b.height;
      parts.push(
        `M${x + r} ${y}H${x + w - r}A${r} ${r} 0 0 1 ${x + w} ${y + r}V${y + h - r}A${r} ${r} 0 0 1 ${x + w - r} ${y + h}` +
          `H${x + r}A${r} ${r} 0 0 1 ${x} ${y + h - r}V${y + r}A${r} ${r} 0 0 1 ${x + r} ${y}Z`,
      );
    }
    const next = `path('${parts.join(" ") || "M0 0Z"}')`;
    if (next !== glassClip) glassClip = next;
  }

  function setRect(r) {
    const round = (v) => Math.round(v * 2) / 2;
    const n = { left: round(r.left), top: round(r.top), width: round(r.width) };
    if (n.left !== rect.left || n.top !== rect.top || n.width !== rect.width) rect = n;
  }

  function measure() {
    if (!shellEl || !mainEl) return;
    const s = shellEl.getBoundingClientRect();
    if (docked) {
      const r = ui.slot.getBoundingClientRect();
      setRect({ left: r.left - s.left, top: r.top - s.top, width: r.width });
    } else {
      const m = mainEl.getBoundingClientRect();
      // top view (~square): large, anchored right, bleeding off the bottom edge
      const width = Math.min(m.width * 0.62, m.height * 0.95);
      setRect({ left: m.left - s.left + m.width - width * 0.85, top: m.top - s.top + m.height * 0.22, width });
    }
    measureGlass(s);
  }

  // Animate only when `docked` flips (tab change). Everything else is untracked: measure()
  // reads rect/scroll every frame and must NOT re-trigger this effect, or scrolling would
  // keep restarting the 750ms transition and the mouse would drift behind the page.
  $effect(() => {
    docked;
    untrack(() => {
      animating = true;
      measure();
    });
    const ids = [80, 320].map((d) => setTimeout(measure, d)); // page fly-in settles
    const done = setTimeout(() => (animating = false), 850);
    return () => [...ids, done].forEach(clearTimeout);
  });
  $effect(() => {
    ui.settled = docked && !animating;
  });

  // Follow layout on scroll, resize, clicks and tab changes (a burst of frames covers the
  // fly-in and the docking transition). measure() only writes state when values change.
  let follow = null;
  onMount(() => {
    follow = followLayout(measure, { scroller: mainEl });
    return () => {
      follow.destroy();
      follow = null;
      ui.settled = false;
    };
  });
  $effect(() => {
    tab;
    const slot = ui.slot;
    // the page under {#key} is replaced on tab change: observe the new one once it is in
    queueMicrotask(() => follow?.observe([shellEl, mainEl, slot, mainEl?.firstElementChild]));
    follow?.burst(1000);
  });
</script>

<div
  class="hero absolute"
  class:docked
  class:animating
  class:handoff={ui.settled}
  class:idle={!vt2.config}
  style="left:{rect.left}px; top:{rect.top}px; width:{rect.width}px"
  aria-hidden={!docked}
  onwheel={(e) => mainEl?.scrollBy({ top: e.deltaY })}
>
  <MouseRender color={prefs.color} selected={ui.sel} {labels} interactive={docked} onselect={(i) => (ui.sel = i)} />
</div>
<!-- frosted copy, visible only through the cards -->
{#if !docked}
  <div class="hero-glass absolute" class:animating style="left:{rect.left}px; top:{rect.top}px; width:{rect.width}px; clip-path:{glassClip}" aria-hidden="true">
    <div class="glass-blur"><MouseRender color={prefs.color} interactive={false} /></div>
  </div>
{/if}

<style>
  .hero {
    z-index: 0;
    opacity: 1;
    pointer-events: none;
    transform-origin: top left;
  }
  .hero.docked {
    z-index: 20;
  }
  .hero.handoff { visibility: hidden; }
  .hero.idle { opacity: 0.18; transition: opacity 400ms ease; }
  /* the art box is wider than the visible crop; only the button shapes take clicks */
  .hero, .hero :global(*) { pointer-events: none; }
  .hero.docked :global([fill^="url"]) { pointer-events: all; }
  .hero-glass {
    z-index: 1;
    pointer-events: none;
    background: var(--background); /* hides the sharp hero underneath the cards */
  }
  .glass-blur { filter: blur(22px) saturate(1.3); }
  .hero-glass :global(*) { pointer-events: none; }
  .hero.animating, .hero-glass.animating {
    transition: left 750ms var(--ease-out-expo), top 750ms var(--ease-out-expo), width 750ms var(--ease-out-expo);
  }
</style>
