<script>
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Kbd } from "$lib/components/ui/kbd/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import MouseRender, { COLORS, CROP, DIY } from "../components/mouse-render.svelte";
  import { onMount } from "svelte";
  import Icon from "$lib/icons/Icon.svelte";
  import { toast } from "svelte-sonner";
  import { vt2, send, BUTTONS, prefs, setColor, ui } from "../store.svelte.js";
  import { ACTIONS, actionKey, actionLabel, same } from "../actions.js";
  import { codeToUsage, usageName, F_EXTRA } from "$lib/hid.js";

  let cfg = $derived(vt2.config);
  let sel = $derived(ui.sel);
  let slotEl;
  onMount(() => {
    ui.slot = slotEl;
    return () => (ui.slot = null);
  });
  let recording = $state(false);
  let pendingLeft = $state(null);

  let cur = $derived(cfg?.buttons[sel]);
  let curKey = $derived(cur ? actionKey(cur) : null);
  let locked = $derived(BUTTONS[sel].locked);
  let labels = $derived(Object.fromEntries(BUTTONS.map((b, i) => [i, `${b.name} · ${cfg ? actionLabel(cfg.buttons[i]) : ""}`])));

  // the DIY button is on the underside: selecting it flips the render; "Virar" toggles back
  let lastTop = 6;
  function select(i) {
    if (i !== DIY) lastTop = i;
    ui.sel = i;
    recording = false;
    pendingLeft = null;
  }

  function assign(value) {
    if (same(value, cur)) return;
    if (sel === 0 && !same(value, { Mouse: 1 }) && pendingLeft === null) {
      pendingLeft = value;
      return;
    }
    pendingLeft = null;
    cfg.buttons[sel] = value;
    send("button", [sel, value]);
  }

  function onKey(e) {
    if (!recording) return;
    e.preventDefault();
    recording = false;
    const u = codeToUsage(e.code);
    if (u === undefined) return toast.error(`Tecla não suportada: ${e.code}`);
    assign({ Key: u });
  }
</script>

<svelte:window onkeydown={onKey} />

{#if cfg}
  <PageHeader title="Botões" description="Escolha um botão no mouse ou no menu e defina o que ele faz." />

  <div class="grid gap-4">
    <Card.Root class="overflow-hidden py-0">
      <div class="relative px-6 pt-5 pb-1">
        <div class="absolute top-4 right-4 z-10 flex items-center gap-1.5">
          {#each COLORS as c}
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  <button
                    {...props}
                    class="size-4 rounded-full border transition-transform hover:scale-110 border-white/25 {prefs.color === c.key ? 'ring-ring ring-offset-card ring-2 ring-offset-2' : ''}"
                    style="background:{c.hex}"
                    aria-label={c.name}
                    onclick={() => setColor(c.key)}
                  ></button>
                {/snippet}
              </Tooltip.Trigger>
              <Tooltip.Content>{c.name}</Tooltip.Content>
            </Tooltip.Root>
          {/each}
        </div>
        <div class="mx-auto max-w-[320px]">
          <div bind:this={slotEl} class="w-full" style="aspect-ratio:{CROP.w}/{CROP.h}">
            {#if ui.settled}
              <MouseRender color={prefs.color} selected={sel} {labels} onselect={select} />
            {/if}
          </div>
        </div>
        <div class="absolute bottom-3 left-4 flex items-center gap-2">
          <Button variant="outline" size="sm" onclick={() => select(sel === DIY ? lastTop : DIY)}>
            <Icon name="refresh" />{sel === DIY ? "Ver de cima" : "Ver embaixo"}
          </Button>
          {#if sel === DIY}<span class="text-muted-foreground text-xs">Parte de baixo · botão DIY</span>{/if}
        </div>
      </div>
    </Card.Root>

    <div>
      <Card.Root>
        <Card.Header>
          <Card.Description>Botão</Card.Description>
          <Select.Root type="single" value={String(sel)} onValueChange={(v) => v && select(+v)}>
            <Select.Trigger class="mt-1 h-10 w-72 text-base font-semibold">
              {BUTTONS[sel].name}
            </Select.Trigger>
            <Select.Content>
              {#each BUTTONS as b, i}
                <Select.Item value={String(i)} label={b.name}>
                  <div class="flex w-full items-center justify-between gap-6">
                    <span>{b.name}</span>
                    <span class="text-muted-foreground text-xs">{actionLabel(cfg.buttons[i])}</span>
                  </div>
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
          <Card.Action><Badge variant="secondary">{actionLabel(cur)}</Badge></Card.Action>
        </Card.Header>
        <Card.Content>
          {#if locked}
            <div class="text-muted-foreground flex gap-3 text-sm">
              <Icon name="lock" class="mt-0.5" />
              A rolagem usa um código de função que ainda não foi decodificado. Por segurança, ela não pode ser alterada aqui.
            </div>
          {:else}
            <div class="grid grid-cols-2 gap-2">
              {#each ACTIONS as a}
                {#if !(sel === 0 && a.key === "off")}
                  <Button variant={curKey === a.key ? "default" : "outline"} class="justify-start" onclick={() => assign(a.value)}>
                    <Icon name={a.icon} />{a.label}
                  </Button>
                {/if}
              {/each}
              <Button
                variant={curKey === "key" ? "default" : "outline"}
                class="col-span-2 justify-start {recording ? 'ring-brand ring-2' : ''}"
                onclick={() => (recording = !recording)}
              >
                <Icon name="keyboard" />
                {#if recording}
                  Pressione uma tecla…
                {:else if curKey === "key"}
                  Tecla <Kbd class="ml-1">{usageName(cur.Key)}</Kbd>
                {:else}
                  Tecla do teclado
                {/if}
              </Button>
            </div>

            <div class="mt-5">
              <div class="text-muted-foreground mb-2 text-xs">Teclas extras F13–F24, que não existem no teclado e são ótimas para atalhos</div>
              <div class="flex flex-wrap gap-1.5">
                {#each F_EXTRA as u}
                  <Button size="sm" variant={curKey === "key" && cur.Key === u ? "secondary" : "ghost"} class="h-7 px-2 font-mono text-xs" onclick={() => assign({ Key: u })}>
                    {usageName(u)}
                  </Button>
                {/each}
              </div>
            </div>

            {#if pendingLeft !== null}
              <div class="border-destructive/40 bg-destructive/10 mt-5 flex items-center gap-3 rounded-lg border p-3 text-sm">
                <Icon name="warning" class="text-destructive" />
                <span class="flex-1">Trocar o clique esquerdo pode deixar o mouse difícil de usar.</span>
                <Button size="sm" variant="destructive" onclick={() => assign(pendingLeft)}>Trocar</Button>
                <Button size="sm" variant="ghost" onclick={() => (pendingLeft = null)}>Cancelar</Button>
              </div>
            {/if}
          {/if}
        </Card.Content>
      </Card.Root>
    </div>
  </div>
{/if}
