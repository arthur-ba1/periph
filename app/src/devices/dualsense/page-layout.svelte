<script>
  // Shared DualSense page frame: compact header, the 3D stage on the left and the settings on
  // the right, both sized to the window so a page fits without scrolling (the settings column
  // scrolls on its own only if a section is taller than the window).
  import ControllerStage from "./controller-stage.svelte";

  /** title, description: header; home: stage focus on open; virtual: emulated twin;
   *  tabs: snippet above the settings (section tabs); children: the settings. */
  let { title, description, home = "overview", virtual = false, tabs = null, children } = $props();
</script>

<div class="flex h-[max(24rem,calc(100vh-44px-4rem))] flex-col gap-4">
  <div class="shrink-0">
    <h1 class="text-xl font-semibold tracking-tight">{title}</h1>
    {#if description}<p class="text-muted-foreground mt-0.5 line-clamp-1 text-[13px]" title={description}>{description}</p>{/if}
  </div>
  <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,4fr)_minmax(0,6fr)] gap-4">
    <div class="min-h-0">
      <ControllerStage {home} {virtual} />
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
