<script>
  import { Switch } from "$lib/components/ui/switch/index.js";
  import PageLayout from "../page-layout.svelte";
  import SectionTabs from "../section-tabs.svelte";
  import TriggerCard from "../trigger-card.svelte";
  import { ds, send, ui, focusPart, flashPart } from "../store.svelte.js";
  import { t } from "$lib/i18n.svelte.js";

  const KEY = "periph.dualsense.linkTriggers";
  /** Link: one effect for both triggers (UI preference; the driver still gets two commands). */
  let linked = $state(globalThis.localStorage?.getItem(KEY) === "1");
  let side = $derived(ui.focus === "l2" ? "l2" : ui.focus === "r2" ? "r2" : "r2");
  let section = $derived(side === "l2" ? "leftTrigger" : "rightTrigger");
  let cfg = $derived(ds.config);
  const active = (sec) => !!cfg?.[sec]?.effect;

  function setLinked(on) {
    linked = on;
    localStorage.setItem(KEY, on ? "1" : "0");
    if (on) {
      // the trigger on screen becomes the effect for both
      const other = section === "leftTrigger" ? "rightTrigger" : "leftTrigger";
      const value = structuredClone($state.snapshot(cfg[section]));
      cfg[other] = value;
      send(other, value);
      flashPart(other === "leftTrigger" ? "l2" : "r2");
      focusPart("triggers");
    } else {
      focusPart(side);
    }
  }

  $effect(() => {
    // keep the stage on both triggers while linked (a callout click may pick one)
    if (linked && (ui.focus === "l2" || ui.focus === "r2")) focusPart("triggers");
  });

  let items = $derived(
    linked
      ? [{ value: "both", label: t("dualsense.triggers.both"), icon: "trigger", on: active(section) }]
      : [
          { value: "l2", label: t("dualsense.triggers.leftShort"), icon: "trigger", on: active("leftTrigger") },
          { value: "r2", label: t("dualsense.triggers.rightShort"), icon: "trigger", flip: true, on: active("rightTrigger") },
        ],
  );
</script>

{#if cfg}
  <PageLayout title={t("dualsense.triggers.title")} home={linked ? "triggers" : side}>
    {#snippet tabs()}
      <SectionTabs {items} value={linked ? "both" : side} onchange={(v) => v !== "both" && focusPart(v)}>
        {#snippet trailing()}
          <label class="text-muted-foreground flex cursor-pointer items-center gap-2 text-xs font-medium whitespace-nowrap">
            {t("dualsense.triggers.link")}
            <Switch checked={linked} onCheckedChange={setLinked} size="sm" />
          </label>
        {/snippet}
      </SectionTabs>
    {/snippet}

    {#key section + linked}
      <TriggerCard {section} {linked} />
    {/key}
  </PageLayout>
{/if}
