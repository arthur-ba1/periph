<script>
  // The controller stage beside every DualSense page: the 3D controller flies to the
  // part being edited and pulses whatever just changed.
  import { onMount } from "svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import Controller3D, { MODEL_URL } from "./controller-3d.svelte";
  import { ds, ui, focusPart } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  /** home: the part this page opens on; virtual: show the emulated twin. */
  let { home = "overview", virtual = false } = $props();
  const callouts = ["lightbar", "player", "mic", "l2", "r2", "triggers"];

  onMount(() => focusPart(home));
</script>

<div class="stage bg-card relative flex h-full min-h-0 flex-col overflow-hidden rounded-xl border">
  <!-- backdrop: soft spotlight that follows the brand tint -->
  <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_50%_35%,color-mix(in_oklch,var(--color-brand)_14%,transparent),transparent_65%)]"></div>

  <!-- focused part name -->
  {#key ui.focus}
    <div class="text-muted-foreground absolute top-3.5 left-4 z-10 text-[11px] font-medium tracking-wide uppercase">
      {t(`dualsense.stage.parts.${ui.focus}`)}
    </div>
  {/key}

  <div class="relative min-h-0 flex-1">
    <Controller3D cfg={ds.config} focus={ui.focus} {virtual} flash={ui.flash} labels={callouts} onselect={focusPart}>
      {#snippet fallback()}
        <div class="text-muted-foreground grid h-full place-items-center px-6 text-center text-xs leading-relaxed">
          <div class="grid justify-items-center gap-3"><Icon name="gamepad" size={64} class="opacity-40" />{t("dualsense.stage.noModel")}</div>
        </div>
      {/snippet}
    </Controller3D>
    {#if MODEL_URL}
      <div class="text-muted-foreground/60 absolute right-3 bottom-1.5 left-3 truncate text-right text-[9.5px]" title={t("dualsense.stage.credit")}>{t("dualsense.stage.credit")}</div>
    {/if}
  </div>
</div>

