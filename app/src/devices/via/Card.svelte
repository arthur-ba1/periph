<script>
  // Sidebar card: keyboard outline, connection, VID/PID. Right click picks the keyboard color
  // the views draw (like the mouse and DualSense cards).
  import { Separator } from "$lib/components/ui/separator/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import { ContextMenu } from "bits-ui";
  import CheckIcon from "@lucide/svelte/icons/check";
  import KeyboardView from "./keyboard-view.svelte";
  import { kbd, prefs, COLORS, setColor, CAPS, setCaps } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";
</script>

<ContextMenu.Root>
<ContextMenu.Trigger class="border-sidebar-border bg-card block rounded-lg border p-3">
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
</ContextMenu.Trigger>
<ContextMenu.Portal>
  <ContextMenu.Content class="bg-popover text-popover-foreground ring-foreground/10 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 z-50 min-w-44 rounded-lg p-1 shadow-md ring-1 duration-100">
    <div class="text-muted-foreground px-1.5 py-1 text-xs">{t("via.card.color")}</div>
    <ContextMenu.RadioGroup value={prefs.color} onValueChange={setColor}>
      {#each COLORS as c (c.key)}
        <ContextMenu.RadioItem value={c.key} class="data-highlighted:bg-accent data-highlighted:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-md py-1 pr-8 pl-1.5 text-sm outline-hidden select-none">
          {#snippet children({ checked })}
            <span class="size-3 rounded-full border border-white/25" style="background:{c.hex}"></span>
            {t(`via.colors.${c.key}`)}
            {#if checked}<CheckIcon class="absolute right-2 size-4" />{/if}
          {/snippet}
        </ContextMenu.RadioItem>
      {/each}
    </ContextMenu.RadioGroup>
    <ContextMenu.Separator class="bg-border -mx-1 my-1 h-px" />
    <div class="text-muted-foreground px-1.5 py-1 text-xs">{t("via.card.caps")}</div>
    <ContextMenu.RadioGroup value={prefs.caps} onValueChange={setCaps}>
      {#each CAPS as c (c)}
        <ContextMenu.RadioItem value={c} class="data-highlighted:bg-accent data-highlighted:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-md py-1 pr-8 pl-1.5 text-sm outline-hidden select-none">
          {#snippet children({ checked })}
            {t(`via.caps.${c}`)}
            {#if checked}<CheckIcon class="absolute right-2 size-4" />{/if}
          {/snippet}
        </ContextMenu.RadioItem>
      {/each}
    </ContextMenu.RadioGroup>
  </ContextMenu.Content>
</ContextMenu.Portal>
</ContextMenu.Root>
