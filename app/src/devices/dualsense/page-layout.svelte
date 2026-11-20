<script>
  // Shared DualSense page frame: compact header, the 3D stage on the left and the settings on
  // the right, both sized to the window so a page fits without scrolling (the settings column
  // scrolls on its own only if a section is taller than the window).
  // The 3D stage itself is not here: it is one persistent layer (stage-layer.svelte) laid over
  // this placeholder, so switching tabs moves the same controller instead of rebuilding it.
  import { onMount } from "svelte";
  import { ui, focusPart } from "./store.svelte.js";

  /** title, description: header; home: stage focus on open; virtual: emulated twin; preview: display-only target;
   *  tabs: snippet above the settings (section tabs); children: the settings. */
  let { title, description, home = "overview", virtual = false, preview = null, tabs = null, children } = $props();

  let slot = $state(null);
  $effect(() => {
    ui.slot = slot;
    return () => ui.slot === slot && (ui.slot = null);
  });
  $effect(() => {
    ui.virtual = virtual;
    ui.preview = preview;
  });
  onMount(() => focusPart(home));
</script>

<div class="flex h-[max(24rem,calc(100vh-44px-4rem))] flex-col gap-4">
  <div class="shrink-0">
    <h1 class="text-xl font-semibold tracking-tight">{title}</h1>
    {#if description}<p class="text-muted-foreground mt-0.5 line-clamp-1 text-[13px]" title={description}>{description}</p>{/if}
  </div>
  <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,4fr)_minmax(0,6fr)] gap-4">
    <div class="min-h-0">
      <div bind:this={slot} class="size-full"></div>
    </div>
    <div class="flex min-h-0 flex-col gap-3">
      {#if tabs}<div class="shrink-0">{@render tabs()}</div>{/if}
      <div class="scroll-slim -mr-2 min-h-0 flex-1 overflow-y-auto pr-2">
        {@render children()}
      </div>
    </div>
  </div>
</div>

<style>
  .scroll-slim { scrollbar-width: thin; scrollbar-color: color-mix(in oklch, var(--color-foreground) 18%, transparent) transparent; }
</style>
