<script>
  // "Override the game" switch shared by every effect section (game wins by default), shown at
  // the top of the section with a "?" that explains it.
  import { Switch } from "$lib/components/ui/switch/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import { t } from "$lib/i18n.svelte.js";

  let { checked, onchange } = $props();
  const id = $props.id();
</script>

<div class="bg-muted/30 flex items-center justify-between gap-3 rounded-lg border px-3 py-2">
  <div class="flex items-center gap-1.5">
    <label for={id} class="cursor-pointer text-[13px] font-medium">{t("dualsense.override.title")}</label>
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            type="button"
            class="text-muted-foreground hover:text-foreground hover:border-foreground/40 grid size-4 place-items-center rounded-full border text-[10px] leading-none font-semibold transition-colors"
            aria-label={t("dualsense.override.help")}
          >?</button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content class="grid max-w-64 gap-2 py-2 text-xs leading-relaxed">
        <p>{t("dualsense.override.desc")}</p>
        <p><strong class="font-semibold">{t("dualsense.override.offLabel")}:</strong> {t("dualsense.override.off")}</p>
        <p><strong class="font-semibold">{t("dualsense.override.onLabel")}:</strong> {t("dualsense.override.on")}</p>
      </Tooltip.Content>
    </Tooltip.Root>
  </div>
  <Switch {id} {checked} onCheckedChange={onchange} />
</div>
