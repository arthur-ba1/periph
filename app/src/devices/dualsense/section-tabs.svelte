<script>
  // Segmented tabs above a page's card: one section visible at a time (no scrolling), with a
  // sliding highlight and a dot on sections whose setting is active.
  import Icon from "$lib/icons/Icon.svelte";

  /** items: [{ value, label, icon?, on? }]; value: selected; onchange(value); trailing: snippet
   *  rendered at the right end of the bar (e.g. the "link triggers" switch). */
  let { items, value, onchange, trailing = null, disabled = false } = $props();

  // tabs share the width equally (gap 4px), so the pill position is pure CSS
  let index = $derived(items.findIndex((it) => it.value === value));
  let n = $derived(items.length);
</script>

<div class="bg-muted/50 flex items-center gap-2 rounded-xl border p-1">
  <div class="relative flex min-w-0 flex-1 gap-1" role="tablist">
    {#if index >= 0 && !disabled}
      <span
        class="bg-background absolute inset-y-0 rounded-lg border shadow-sm transition-[left,width] duration-300 ease-(--ease-out-expo)"
        style:width="calc((100% - {(n - 1) * 4}px) / {n})"
        style:left="calc((100% - {(n - 1) * 4}px) / {n} * {index} + {index * 4}px)"
      ></span>
    {/if}
    {#each items as it (it.value)}
      <button
        type="button"
        role="tab"
        aria-selected={value === it.value}
        {disabled}
        class="relative flex h-8 min-w-0 flex-1 items-center justify-center gap-1.5 rounded-lg px-2 text-[13px] font-medium transition-colors disabled:pointer-events-none
          {value === it.value && !disabled ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => onchange?.(it.value)}
      >
        {#if it.icon}<Icon name={it.icon} size={15} class={it.flip ? "-scale-x-100" : ""} />{/if}
        <span class="truncate">{it.label}</span>
        {#if it.on}<span class="bg-brand absolute top-1.5 right-1.5 size-1.5 rounded-full shadow-[0_0_6px_var(--color-brand)]"></span>{/if}
      </button>
    {/each}
  </div>
  {#if trailing}
    <div class="shrink-0 pr-1.5">{@render trailing()}</div>
  {/if}
</div>
