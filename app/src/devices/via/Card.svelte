<script>
  // Sidebar card: keyboard outline, connection, VID/PID.
  import { Separator } from "$lib/components/ui/separator/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import KeyboardView from "./keyboard-view.svelte";
  import { kbd } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";
</script>

<div class="border-sidebar-border bg-card rounded-lg border p-3">
  <div class="mb-3 flex h-20 items-center justify-center overflow-hidden rounded-md bg-gradient-to-b from-white/[0.04] to-transparent">
    {#if kbd.definition}
      <div class="pointer-events-none origin-center scale-[0.95] {kbd.connected ? '' : 'opacity-40 grayscale'}">
        <KeyboardView def={kbd.definition} keymap={null} unit={11} />
      </div>
    {:else}
      <Icon name="keyboard" size={34} class="text-muted-foreground" />
    {/if}
  </div>
  <div class="flex items-center justify-between">
    <span class="truncate text-sm font-medium">{kbd.definition?.displayName ?? kbd.definition?.name ?? (kbd.product || t("via.name"))}</span>
    <span class="flex items-center gap-1.5 text-xs {kbd.connected ? 'text-success' : 'text-warning'}">
      <span class="relative inline-flex size-2 rounded-full {kbd.connected ? 'bg-success' : 'bg-warning'}"></span>
      {t(kbd.connected ? "via.card.online" : "via.card.searching")}
    </span>
  </div>
  {#if kbd.connected}
    <Separator class="my-3" />
    <div class="text-muted-foreground grid gap-1 font-mono text-[11px]">
      <span>VID {kbd.vendorId.toString(16).toUpperCase().padStart(4, "0")} · PID {kbd.productId.toString(16).toUpperCase().padStart(4, "0")}</span>
      <span>{t("via.card.layers", { protocol: kbd.protocol, layers: kbd.layers })}</span>
    </div>
  {/if}
</div>
