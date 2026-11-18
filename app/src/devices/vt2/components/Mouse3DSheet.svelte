<script>
  // Dev-only preview: open the UI with `?3d`.
  import Mouse3D, { MODEL_URL } from "./mouse-3d.svelte";
  import { COLORS } from "./mouse-render.svelte";

  let color = $state(COLORS.find((c) => c.key === 4)?.hex ?? "#FF2558");
  let auto = $state(true);
</script>

<div class="bg-background text-foreground flex h-screen flex-col gap-4 p-6">
  <div class="flex items-center gap-3">
    <h1 class="text-lg font-semibold tracking-tight">VT2 Max 3D</h1>
    <code class="text-muted-foreground text-xs">{MODEL_URL ?? "sem model.glb"}</code>
    <div class="ml-auto flex items-center gap-2">
      {#each COLORS as c}
        <button class="size-5 rounded-full border border-white/25 {color === c.hex ? 'ring-ring ring-2 ring-offset-2 ring-offset-background' : ''}" style="background:{c.hex}" aria-label={c.name} onclick={() => (color = c.hex)}></button>
      {/each}
      <button class="border-border ml-3 rounded-md border px-3 py-1 text-xs" onclick={() => (auto = !auto)}>{auto ? "Pausar giro" : "Girar"}</button>
    </div>
  </div>
  <div class="border-border bg-card relative flex-1 rounded-xl border">
    <Mouse3D {color} autoRotate={auto} />
  </div>
</div>
