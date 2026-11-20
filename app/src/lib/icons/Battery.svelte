<script module>
  /** Shared reading of a battery level: "charging" | "full" | "low" | "ok" | null (unknown). */
  export function batteryState(level, charging = false, full = false) {
    if (level == null) return null;
    if (charging) return "charging";
    if (full) return "full";
    return level <= 20 ? "low" : "ok";
  }
  /** Text colour per state (glyph, labels); progress bars use BATTERY_BAR. */
  export const BATTERY_TONE = { charging: "text-success", full: "text-success", low: "text-destructive", ok: "", null: "" };
  export const BATTERY_BAR = {
    charging: "charging-bar [&_[data-slot=progress-indicator]]:bg-success",
    full: "[&_[data-slot=progress-indicator]]:bg-success",
    low: "[&_[data-slot=progress-indicator]]:bg-destructive",
    ok: "",
    null: "",
  };
</script>

<script>
  // Battery glyph: segmented fill (4 cells). Charging: bolt over the cells. Low (≤ 20 %): one
  // cell, and an exclamation mark instead once it is nearly empty (≤ 10 %). Unknown: "?"-like
  // mark. Colours itself by state unless `tone` is false.
  let { level = null, charging = false, full = false, tone = true, size = 16, class: className = "" } = $props();
  let state = $derived(batteryState(level, charging, full));
  let cells = $derived(level == null ? 0 : Math.max(1, Math.ceil(level / 25)));
  let empty = $derived(state === "low" && level <= 10);
</script>

<svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" class="shrink-0 {tone ? BATTERY_TONE[state] : ''} {className}" aria-hidden="true">
  <rect x="2.75" y="7.25" width="16.5" height="9.5" rx="2.5" />
  <path d="M21.25 10.5v3" />
  {#if level == null && !charging}
    <path d="M11 9.75v3M11 15h.01" />
  {:else if charging}
    {#each Array(cells) as _, i}
      <rect x={5 + i * 3.25} y="9.5" width="2.25" height="5" rx="0.6" fill="currentColor" stroke="none" opacity="0.3" />
    {/each}
    <path d="M11.9 8.6 9.2 12.3h3.4l-2.7 3.7" stroke-width="1.7" class="charge" />
  {:else if empty}
    <path d="M11 9.6v3.1M11 14.9h.01" />
  {:else}
    {#each Array(cells) as _, i}
      <rect x={5 + i * 3.25} y="9.5" width="2.25" height="5" rx="0.6" fill="currentColor" stroke="none" />
    {/each}
  {/if}
</svg>

<style>
  .charge { animation: charge 1.6s ease-in-out infinite; }
  @keyframes charge { 0%, 100% { opacity: 1; } 50% { opacity: 0.45; } }
  @media (prefers-reduced-motion: reduce) { .charge { animation: none; } }
</style>
