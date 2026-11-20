<script>
  // The controller stage beside every DualSense page (placed by stage-layer.svelte): the 3D
  // controller flies to the part being edited and pulses whatever just changed.
  import Icon from "$lib/icons/Icon.svelte";
  import Controller3D, { MODEL_URL } from "./controller-3d.svelte";
  import GhostPreview from "./ghost-preview.svelte";
  import { EMULATION_MODELS } from "./emulation-models.js";
  import { ds, ui, prefs, focusPart } from "./store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  const callouts = ["lightbar", "player", "mic", "l2", "r2", "triggers"];
  let preview = $derived(ui.preview);
  let sweptPreview = $derived(preview && MODEL_URL && EMULATION_MODELS[preview].url ? preview : null);
  let credit = $derived(preview ? (EMULATION_MODELS[preview].url ? t(`dualsense.stage.modelCredits.${preview}`) : "") : MODEL_URL ? t("dualsense.stage.credit") : "");
</script>

<div class="stage bg-card relative flex h-full min-h-0 flex-col overflow-hidden rounded-xl border">
  <!-- backdrop: soft spotlight that follows the brand tint -->
  <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_50%_35%,color-mix(in_oklch,var(--color-brand)_14%,transparent),transparent_65%)]"></div>

  <!-- focused part name -->
  {#key preview ?? ui.focus}
    <div class="text-muted-foreground absolute top-3.5 left-4 z-10 text-[11px] font-medium tracking-wide uppercase">
      {preview ? t(`dualsense.emulation.models.${preview}`) : t(`dualsense.stage.parts.${ui.focus}`)}
    </div>
  {/key}

  <div class="relative min-h-0 flex-1">
    {#snippet fallback()}
      <div class="text-muted-foreground grid h-full place-items-center px-6 text-center text-xs leading-relaxed">
        <div class="grid justify-items-center gap-3"><Icon name="gamepad" size={64} class="opacity-40" />{t("dualsense.stage.noModel")}</div>
      </div>
    {/snippet}
    <div class="size-full" hidden={!!preview && !sweptPreview}>
      <Controller3D cfg={ds.config} lights={ds.connected ? ds.status?.lights : null} controls={ds.connected ? ds.status?.controls : null} model={prefs.model} focus={ui.focus} virtual={ui.virtual} preview={sweptPreview} flash={ui.flash} labels={callouts} onselect={focusPart} {fallback} />
    </div>
    {#if preview && !sweptPreview}
      {#key preview}
        <GhostPreview model={preview} asset={EMULATION_MODELS[preview]} controls={ds.connected ? ds.status?.controls : null} {fallback} />
      {/key}
    {/if}
    {#if credit}
      <div class="text-muted-foreground/60 absolute right-3 bottom-1.5 left-3 truncate text-right text-[9.5px]" title={credit}>{credit}</div>
    {/if}
  </div>
</div>

