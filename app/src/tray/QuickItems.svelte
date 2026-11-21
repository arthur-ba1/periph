<script>
  // A device's quick settings (the driver's `Tray` items): a Check is a switch, a Submenu of
  // Checks a choice group (Separators in it split the row into groups), Colors a row of
  // swatches, a Slider a range, an Action a button. Long choice groups start folded to the
  // chosen option. Used by the flyout and by the right-click menu.
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import { command } from "./api.js";

  /** `width`: inner width in px, to size the choice groups. */
  let { device, items, width = 288, compact = false } = $props();

  const isChoice = (item) =>
    item.type === "submenu" && item.items.some((i) => i.type === "check") && item.items.every((i) => i.type === "check" || i.type === "separator");
  const optionsOf = (group) => group.items.filter((i) => i.type === "check");
  /** Runs of options between separators. */
  function groupsOf(group) {
    const out = [[]];
    for (const i of group.items) i.type === "separator" ? out.push([]) : out.at(-1).push(i);
    return out.filter((g) => g.length);
  }
  // One row when every option fits; otherwise as few rows as possible with the same number of
  // options each (the last one centred), so groups never break unevenly.
  const GAP = 2, PAD = 2, CHIP_PAD = 14;
  let ctx;
  function textWidth(text) {
    ctx ??= document.createElement("canvas").getContext("2d");
    ctx.font = `500 12px ${getComputedStyle(document.body).fontFamily}`;
    return ctx.measureText(text).width;
  }
  function layout(options) {
    const inner = width - PAD * 2;
    const chip = Math.max(...options.map((o) => textWidth(o.label))) + CHIP_PAD;
    const fit = Math.max(1, Math.floor((inner + GAP) / (chip + GAP)));
    const rows = Math.ceil(options.length / fit);
    const perRow = Math.ceil(options.length / rows);
    return { rows, basis: `calc((100% - ${(perRow - 1) * GAP}px) / ${perRow})` };
  }

  // Optimistic: reflect the click now, the backend's next payload confirms it.
  function choose(group, option) {
    for (const o of optionsOf(group)) o.checked = o === option;
    open[group.label] = false; // a long list folds back once something is picked
    command(device.id, option.id);
  }
  function pickColor(group, option) {
    for (const o of group.options) o.checked = o === option;
    command(device.id, option.id);
  }
  const rgb = (c) => `rgb(${c.join(",")})`;

  // choice groups with more options than this start folded: one row showing the current choice
  const FOLD = 6;
  let open = $state({}); // label -> unfolded
  // slider drafts while dragging; the command goes out on release
  let drafts = $state({});
  function slide(entry, v) {
    delete drafts[entry.id];
    entry.value = v;
    command(device.id, `${entry.id}:${v}`);
  }
  function toggle(item) {
    item.checked = !item.checked;
    command(device.id, item.id);
  }

  const title = "text-muted-foreground text-[11px] font-medium tracking-wide uppercase";
</script>

{#snippet choice(group, option, flex)}
  <button
    style:flex
    class="tabular min-w-0 truncate rounded-md px-1.5 text-xs font-medium transition-colors {compact ? 'h-6' : 'h-7'} {option.checked
      ? 'bg-foreground text-background'
      : 'text-muted-foreground hover:bg-accent hover:text-foreground'}"
    title={option.label}
    onclick={() => choose(group, option)}
  >
    {option.label}
  </button>
{/snippet}

{#snippet item(entry)}
  {#if isChoice(entry)}
    {@const options = optionsOf(entry)}
    {@const groups = groupsOf(entry)}
    {@const fit = layout(options)}
    {@const folded = options.length > FOLD && !open[entry.label]}
    <section class="flex flex-col gap-1.5">
      {#if options.length > FOLD}
        <button class="flex items-center justify-between text-left" aria-expanded={!folded} onclick={() => (open[entry.label] = !open[entry.label])}>
          <span class={title}>{entry.label}</span>
          <span class="text-muted-foreground flex min-w-0 items-center gap-1 text-xs">
            {#if folded}<span class="truncate">{options.find((o) => o.checked)?.label ?? ""}</span>{/if}
            <ChevronDown class="size-3.5 shrink-0 transition-transform {folded ? '' : 'rotate-180'}" />
          </span>
        </button>
      {:else}
        <span class={title}>{entry.label}</span>
      {/if}
      {#if folded}
        <!-- folded: nothing more; the header shows the current choice -->
      {:else if groups.length > 1 && fit.rows === 1}
        <!-- separated groups on one line, every option the same width -->
        <div class="flex items-center gap-1.5">
          {#each groups as group, g}
            {#if g}<div class="bg-border h-4 w-px shrink-0"></div>{/if}
            <div class="bg-muted/60 flex gap-0.5 rounded-lg p-0.5" style:flex="{group.length} 1 0">
              {#each group as option}{@render choice(entry, option, "1 1 0")}{/each}
            </div>
          {/each}
        </div>
      {:else if groups.length > 1}
        <!-- too wide for one line: one line per group, never mixing them -->
        {#each groups as group}
          <div class="bg-muted/60 flex gap-0.5 rounded-lg p-0.5">
            {#each group as option}{@render choice(entry, option, "1 1 0")}{/each}
          </div>
        {/each}
      {:else}
        <div class="bg-muted/60 flex flex-wrap justify-center gap-0.5 rounded-lg p-0.5" style:--basis={fit.basis}>
          {#each options as option}{@render choice(entry, option, "0 0 var(--basis)")}{/each}
        </div>
      {/if}
    </section>
  {:else if entry.type === "colors"}
    <section class="flex flex-col gap-1.5">
      <span class={title}>{entry.label}</span>
      <div class="flex items-center justify-between gap-1">
        {#each entry.options as option}
          <button
            class="rounded-full border border-white/20 transition-transform hover:scale-110 {compact ? 'size-5' : 'size-6'} {option.checked ? 'ring-foreground ring-offset-popover ring-2 ring-offset-2' : ''}"
            style="background:{rgb(option.rgb)}"
            aria-label={rgb(option.rgb)}
            aria-pressed={option.checked}
            onclick={() => pickColor(entry, option)}
          ></button>
        {/each}
      </div>
    </section>
  {:else if entry.type === "slider"}
    {@const v = drafts[entry.id] ?? entry.value}
    <section class="flex flex-col gap-2">
      <div class="flex items-center justify-between">
        <span class={title}>{entry.label}</span>
        <span class="text-muted-foreground tabular font-mono text-xs">{v}{entry.unit}</span>
      </div>
      <Slider type="single" min={entry.min} max={entry.max} step={entry.step || 1} value={v}
        onValueChange={(x) => (drafts[entry.id] = x)} onValueCommit={(x) => slide(entry, x)} />
    </section>
  {:else if entry.type === "submenu"}
    <section class="flex flex-col gap-1">
      <span class={title}>{entry.label}</span>
      {#each entry.items as child}{@render item(child)}{/each}
    </section>
  {:else if entry.type === "check"}
    <label class="hover:bg-accent/60 -mx-2 flex cursor-pointer items-center justify-between rounded-md px-2 {compact ? 'h-8 text-[13px]' : 'h-9 text-sm'}">
      {entry.label}
      <Switch checked={entry.checked} size={compact ? "sm" : "default"} onCheckedChange={() => toggle(entry)} />
    </label>
  {:else if entry.type === "action"}
    <button class="hover:bg-accent/60 -mx-2 flex items-center rounded-md px-2 text-left {compact ? 'h-8 text-[13px]' : 'h-9 text-sm'}" onclick={() => command(device.id, entry.id)}>
      {entry.label}
    </button>
  {:else if entry.type === "separator"}
    <div class="bg-border h-px"></div>
  {/if}
{/snippet}

<div class="flex flex-col {compact ? 'gap-2' : 'gap-3'}">
  {#each items as entry}{@render item(entry)}{/each}
</div>
