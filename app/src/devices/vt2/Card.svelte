<script>
  // Sidebar card: render thumbnail, connection, battery.
  import { fade } from "svelte/transition";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import BatteryGlyph from "$lib/icons/Battery.svelte";
  import { vt2, prefs } from "./store.svelte.js";

  const thumbs = import.meta.glob("./assets/thumb_*.png", { eager: true, import: "default" });

  let st = $derived(vt2.status);
  let battery = $derived(st?.battery ?? null);
  let asleep = $derived(!vt2.connected && (vt2.reason ?? "").includes("dormindo"));
  let thumb = $derived(thumbs[`./assets/thumb_${prefs.color}.png`]);
</script>

<div class="border-sidebar-border bg-card rounded-lg border p-3">
  <div class="relative mb-3 flex h-28 items-center justify-center overflow-hidden rounded-md bg-gradient-to-b from-white/[0.04] to-transparent">
    {#if thumb}
      {#key prefs.color}
        <img src={thumb} alt="" class="h-24 drop-shadow-[0_8px_16px_rgba(0,0,0,0.6)] transition-opacity {vt2.connected ? '' : 'opacity-40 grayscale'}" in:fade={{ duration: 300 }} draggable="false" />
      {/key}
    {:else}
      <Icon name="mouse" size={40} class="text-muted-foreground" />
    {/if}
  </div>
  <div class="flex items-center justify-between">
    <span class="text-sm font-medium">Rapoo VT2 Max</span>
    <span class="flex items-center gap-1.5 text-xs {vt2.connected ? 'text-success' : 'text-warning'}">
      <span class="relative flex size-2">
        {#if !vt2.connected}<span class="bg-warning absolute inline-flex size-full animate-ping rounded-full opacity-75"></span>{/if}
        <span class="relative inline-flex size-2 rounded-full {vt2.connected ? 'bg-success' : 'bg-warning'}"></span>
      </span>
      {vt2.connected ? (st?.wired ? "Cabo" : "Sem fio") : asleep ? "Dormindo" : "Procurando"}
    </span>
  </div>
  <Separator class="my-3" />
  <div class="flex items-center justify-between text-xs">
    <span class="text-muted-foreground flex items-center gap-1.5"><BatteryGlyph level={battery} charging={st?.charging} class={st?.charging ? "text-success" : battery != null && battery <= 20 ? "text-destructive" : ""} />{st?.charging ? "Carregando" : "Bateria"}</span>
    <span class="font-mono font-medium">{battery == null ? "—" : `${battery}%`}</span>
  </div>
  <Progress value={battery ?? 0} class="mt-2 h-1.5" />
  {#if vt2.serial}
    <div class="text-muted-foreground mt-3 font-mono text-[11px]">S/N {vt2.serial}</div>
  {/if}
</div>
