<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Kbd } from "$lib/components/ui/kbd/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import KeyboardView, { fitUnit } from "../keyboard-view.svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import { toast } from "svelte-sonner";
  import { kbd, sendKbd } from "../store.svelte.js";
  import { describe, categories, keycodeFromEvent } from "../keycodes.js";
  import { t } from "$lib/i18n.svelte.js";

  let def = $derived(kbd.definition);
  let layer = $state(0);
  let selKey = $state(null); // "row,col"
  let selInfo = $state(null); // KLE key
  let target = $state("key"); // "key" | "ccw" | "cw"
  let recording = $state(false);
  let cat = $state("basic");
  let boxW = $state(700);
  let unit = $derived(def ? fitUnit(def, boxW, 30, 48) : 44);

  let cats = $derived(categories(kbd.layers));
  let isEncoder = $derived(selInfo?.encoder != null);
  let current = $derived.by(() => {
    if (!selInfo) return null;
    if (target === "key" || !isEncoder) return kbd.keymap[layer]?.[selInfo.row]?.[selInfo.col] ?? 0;
    return kbd.encoders[layer]?.[selInfo.encoder]?.[target === "cw" ? 1 : 0] ?? 0;
  });

  function select(key, info) {
    selKey = key;
    selInfo = info;
    target = "key";
    recording = false;
  }

  function assign(code) {
    if (!selInfo || code === current) return;
    if (target !== "key" && isEncoder) {
      const cw = target === "cw";
      kbd.encoders[layer][selInfo.encoder][cw ? 1 : 0] = code;
      sendKbd({ type: "setEncoder", layer, encoder: selInfo.encoder, clockwise: cw, code });
    } else {
      kbd.keymap[layer][selInfo.row][selInfo.col] = code;
      sendKbd({ type: "setKeycode", layer, row: selInfo.row, col: selInfo.col, code });
    }
  }

  function onKey(e) {
    if (!recording) return;
    e.preventDefault();
    recording = false;
    const code = keycodeFromEvent(e);
    if (code === undefined) return toast.error(t("common.unsupportedKey", { key: e.code }));
    assign(code);
  }
</script>

<svelte:window onkeydown={onKey} />

{#if def}
  <PageHeader title={t("via.keymap.title")} description={t("via.keymap.description")} />

  <div class="grid gap-4">
    <Card.Root>
      <Card.Header>
        <Card.Title>{def.displayName ?? def.name}</Card.Title>
        <Card.Description>{t("via.keymap.layersInfo", { n: kbd.layers })}</Card.Description>
      </Card.Header>
      <Card.Content class="grid gap-5">
        <ToggleGroup.Root type="single" variant="outline" value={String(layer)} onValueChange={(v) => v && (layer = +v)} class="w-full">
          {#each Array(kbd.layers) as _, i}
            <ToggleGroup.Item value={String(i)} class="flex-1 font-mono text-xs">{t("via.keymap.layer", { n: i })}</ToggleGroup.Item>
          {/each}
        </ToggleGroup.Root>
        <div class="pb-1" bind:clientWidth={boxW}>
          {#key layer}
            <KeyboardView {def} keymap={kbd.keymap[layer]} encoders={kbd.encoders[layer]} selected={selKey} onselect={select} {unit} />
          {/key}
        </div>
      </Card.Content>
    </Card.Root>

    {#if selInfo}
      <Card.Root>
        <Card.Header>
          <Card.Description>{t("via.keymap.selected", { layer, key: selKey })}</Card.Description>
          <Card.Title class="flex items-center gap-2 text-xl">
            {describe(current, def).long}
          </Card.Title>
          <Card.Action><Badge variant="secondary" class="font-mono">0x{current.toString(16).toUpperCase().padStart(4, "0")}</Badge></Card.Action>
        </Card.Header>
        <Card.Content class="grid gap-4">
          {#if isEncoder}
            <ToggleGroup.Root type="single" variant="outline" value={target} onValueChange={(v) => v && (target = v)}>
              <ToggleGroup.Item value="key" class="px-4">{t("via.keymap.knobPress")}</ToggleGroup.Item>
              <ToggleGroup.Item value="ccw" class="px-4">{t("via.keymap.knobCcw", { key: describe(kbd.encoders[layer]?.[selInfo.encoder]?.[0] ?? 0, def).short })}</ToggleGroup.Item>
              <ToggleGroup.Item value="cw" class="px-4">{t("via.keymap.knobCw", { key: describe(kbd.encoders[layer]?.[selInfo.encoder]?.[1] ?? 0, def).short })}</ToggleGroup.Item>
            </ToggleGroup.Root>
          {/if}

          <div class="flex items-center gap-2">
            <Button variant={recording ? "default" : "outline"} class={recording ? "ring-brand ring-2" : ""} onclick={() => (recording = !recording)}>
              <Icon name="keyboard" />{recording ? t("common.pressKey") : t("via.keymap.record")}
            </Button>
            <Button variant="ghost" onclick={() => assign(0x01)}>{t("via.keymap.transparent")}</Button>
            <Button variant="ghost" onclick={() => assign(0x00)}>{t("via.keymap.none")}</Button>
          </div>

          <Tabs.Root bind:value={cat}>
            <Tabs.List>
              {#each cats as c}<Tabs.Trigger value={c.id}>{c.label}</Tabs.Trigger>{/each}
            </Tabs.List>
            {#each cats as c}
              <Tabs.Content value={c.id} class="grid gap-4 pt-3">
                {#each c.groups as g}
                  <div>
                    <div class="text-muted-foreground mb-1.5 text-xs">{g.label}</div>
                    <div class="flex flex-wrap gap-1.5">
                      {#each g.codes as code}
                        {@const d = describe(code, def)}
                        <Button
                          size="sm"
                          variant={code === current ? "default" : "outline"}
                          class="h-8 min-w-9 px-2 font-mono text-xs"
                          title={d.long}
                          onclick={() => assign(code)}
                        >{d.short || "—"}</Button>
                      {/each}
                    </div>
                  </div>
                {/each}
              </Tabs.Content>
            {/each}
          </Tabs.Root>
        </Card.Content>
      </Card.Root>
    {:else}
      {@const [before, after] = t("via.keymap.hint").split("{symbol}")}
      <p class="text-muted-foreground text-center text-sm">{before}<Kbd>▽</Kbd>{after}</p>
    {/if}
  </div>
{/if}
