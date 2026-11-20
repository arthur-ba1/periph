<script>
  // Root of the tray window (label "tray"): renders the panel the backend asks for, reports its
  // size so the window can be placed next to the tray, and closes on Escape.
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { i18n, match } from "$lib/i18n.svelte.js";
  import QuickPanel from "./QuickPanel.svelte";
  import TrayMenu from "./TrayMenu.svelte";
  import { TAURI, hide, layout } from "./api.js";
  import { demoPanel } from "./demo.js";

  let panel = $state(null);
  let box = $state();

  function measure() {
    if (!panel || !box) return;
    const { width, height } = box.getBoundingClientRect();
    layout(panel.seq, width, height);
  }

  async function show(next) {
    // follow the backend's language (the main window may have changed it)
    if (next.locale && match(next.locale) !== i18n.locale) i18n.locale = match(next.locale);
    panel = next;
    await tick();
    measure();
  }

  onMount(() => {
    document.documentElement.classList.add("dark");
    if (!TAURI) {
      panel = demoPanel(location.search);
      return;
    }
    const unlisten = listen("tray-panel", ({ payload }) => show(payload));
    const unlistenHidden = listen("tray-panel-hidden", () => (panel = null));
    invoke("flyout_state").then((p) => p && show(p)).catch(() => {});
    // live updates (battery, a setting changed) can change the height
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    return () => {
      unlisten.then((off) => off());
      unlistenHidden.then((off) => off());
      observer.disconnect();
    };
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && hide()} />

<div bind:this={box} class="bg-popover text-popover-foreground absolute top-0 left-0 overflow-hidden {panel?.mode === 'menu' ? 'w-[284px]' : 'w-[320px]'}">
  {#if panel}
    {#key panel.seq}
      <div in:fly={{ y: 6, duration: 240, easing: expoOut }}>
        {#if panel.mode === "menu"}
          <TrayMenu {panel} />
        {:else}
          <QuickPanel {panel} />
        {/if}
      </div>
    {/key}
  {/if}
</div>

<style>
  :global(body) {
    background: var(--popover);
  }
</style>
