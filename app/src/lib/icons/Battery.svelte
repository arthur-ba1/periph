<script>
  // VT2 Icons battery: segmented fill (4 cells); bolt while charging.
  let { level = null, charging = false, size = 16, class: className = "" } = $props();
  let cells = $derived(level == null ? 0 : Math.max(1, Math.ceil(level / 25)));
</script>

<svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" class="shrink-0 {className}" aria-hidden="true">
  <rect x="2.75" y="7.25" width="16.5" height="9.5" rx="2.5" />
  <path d="M21.25 10.5v3" />
  {#if charging}
    <path d="M11.9 8.9 9.4 12.4h3.2l-2.5 3.5" stroke-width="1.6" />
  {:else if level == null}
    <path d="M11 9.75v3M11 15h.01" />
  {:else}
    {#each Array(cells) as _, i}
      <rect x={5 + i * 3.25} y="9.5" width="2.25" height="5" rx="0.6" fill="currentColor" stroke="none" />
    {/each}
  {/if}
</svg>
