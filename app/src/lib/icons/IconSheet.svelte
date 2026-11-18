<script>
  // Dev-only review sheet: open the UI with `?icons`.
  import Icon from "./Icon.svelte";
  import Battery from "./Battery.svelte";
  import { ICONS } from "./icons.js";
</script>

<div class="bg-background text-foreground h-full overflow-auto p-8">
  <h1 class="text-xl font-semibold tracking-tight">VT2 Icons</h1>
  <p class="text-muted-foreground mb-6 text-sm">Grade 24 px · traço 1,75 · pontas arredondadas</p>
  <div class="grid grid-cols-6 gap-3">
    {#each Object.keys(ICONS) as name}
      <div class="border-border bg-card flex flex-col items-center gap-3 rounded-lg border p-4">
        <div class="flex items-end gap-4">
          <Icon {name} size={48} strokeWidth={1.5} />
          <Icon {name} size={24} />
          <Icon {name} size={16} />
        </div>
        <code class="text-muted-foreground text-[11px]">{name}</code>
      </div>
    {/each}
    <div class="border-border bg-card flex flex-col items-center gap-3 rounded-lg border p-4">
      <div class="flex items-end gap-2">
        <Battery level={null} size={32} /><Battery level={20} size={32} /><Battery level={55} size={32} /><Battery level={100} size={32} />
      </div>
      <code class="text-muted-foreground text-[11px]">battery</code>
    </div>
  </div>
</div>
