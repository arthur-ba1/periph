<script>
  // App shell: title bar with the device picker, sidebar with the device's tabs and card,
  // and the active page. Everything device-specific comes from the module in src/devices/<id>/.
  import { onMount } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { expoOut } from "svelte/easing";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";
  import { Toaster } from "$lib/components/ui/sonner/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import Icon from "$lib/icons/Icon.svelte";
  import { MODULES, devices, displayName, initDevices, send } from "$lib/devices.svelte.js";

  // which device is being configured (persisted; "mouse"/"keyboard" are pre-registry ids)
  const LEGACY = { mouse: "vt2", keyboard: "via" };
  const saved = globalThis.localStorage?.getItem("periph.device") ?? LEGACY[globalThis.localStorage?.getItem("vt2.device")];
  let deviceId = $state(MODULES.some((m) => m.id === saved) ? saved : MODULES[0]?.id);
  $effect(() => localStorage.setItem("periph.device", deviceId));

  let mod = $derived(MODULES.find((m) => m.id === deviceId));
  let dev = $derived(devices[deviceId]);
  let tabs = $derived(mod?.tabs ?? []);
  let tabByDevice = $state({});
  let tab = $derived(Math.min(tabByDevice[deviceId] ?? 0, Math.max(tabs.length - 1, 0)));
  let Page = $derived(tabs[tab]?.page);
  let ready = $derived(mod ? (mod.ready?.(dev) ?? !!dev.config) : false);
  let waiting = $derived(
    mod?.waiting?.(dev) ?? {
      title: dev?.connected ? "Lendo configuração…" : `Procurando ${mod?.name ?? "dispositivo"}`,
      text: "Conecte o dispositivo. Feche o software oficial do fabricante, porque os dois não podem usar o dispositivo ao mesmo tempo.",
    },
  );

  let win;
  onMount(() => {
    document.documentElement.classList.add("dark");
    initDevices();
    try { win = getCurrentWindow(); } catch {}
  });

  let shellEl = $state();
  let mainEl = $state();
</script>

<Tooltip.Provider delayDuration={300}>
  <div bind:this={shellEl} class="relative isolate grid h-full grid-cols-[232px_1fr] grid-rows-[44px_1fr] overflow-hidden">
    <!-- optional device background layer (e.g. the VT2 hero render) -->
    {#if mod?.Layer}
      {#key deviceId}
        <mod.Layer {shellEl} {mainEl} {tab} />
      {/key}
    {/if}

    <!-- title bar -->
    <header class="border-border bg-background/70 relative z-10 col-span-2 flex backdrop-blur-md items-center justify-between border-b pl-4" data-tauri-drag-region>
      <div class="flex items-center gap-2 text-sm" data-tauri-drag-region>
        <div class="bg-foreground text-background grid size-5 place-items-center rounded-[5px]"><Icon name="logo" size={14} strokeWidth={2} /></div>
        <!-- device picker -->
        <Select.Root type="single" value={deviceId} onValueChange={(v) => v && (deviceId = v)}>
          <Select.Trigger size="sm" class="hover:bg-accent h-8 gap-2 border-none bg-transparent px-2 font-semibold shadow-none dark:bg-transparent">
            <Icon name={mod?.icon ?? "logo"} size={15} />
            {mod ? displayName(mod) : "Nenhum dispositivo"}
            <span class="size-1.5 rounded-full {dev?.connected ? 'bg-success' : 'bg-muted-foreground/40'}"></span>
          </Select.Trigger>
          <Select.Content class="min-w-60">
            {#each MODULES as m}
              <Select.Item value={m.id} label={displayName(m)}>
                <div class="flex w-full items-center gap-2.5">
                  <Icon name={m.icon} size={16} />
                  <div class="flex flex-col">
                    <span class="font-medium">{displayName(m)}</span>
                    <span class="text-muted-foreground text-xs">{m.kind} · {devices[m.id].connected ? "conectado" : "desconectado"}</span>
                  </div>
                </div>
              </Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>
      </div>
      <div class="flex h-full items-center">
        {#if mod?.Badges}
          <div class="mr-3 flex items-center gap-1.5" in:fade={{ duration: 200 }}>
            <mod.Badges />
          </div>
        {/if}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <Button {...props} variant="ghost" size="icon-sm" class="mr-2" disabled={!mod} onclick={() => send(deviceId, { type: "refresh" })} aria-label="Reler do dispositivo">
                <Icon name="refresh" class={dev?.busy ? "animate-spin" : ""} />
              </Button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content>Reler configuração do dispositivo</Tooltip.Content>
        </Tooltip.Root>
        <Separator orientation="vertical" class="h-5" />
        <button class="text-muted-foreground hover:bg-accent hover:text-foreground grid h-full w-11 place-items-center transition-colors" onclick={() => win?.minimize()} aria-label="Minimizar"><Icon name="minimize" /></button>
        <button class="text-muted-foreground hover:bg-destructive grid h-full w-11 place-items-center transition-colors hover:text-white" onclick={() => invoke("hide_window").catch(() => win?.hide())} aria-label="Fechar para a bandeja"><Icon name="close" /></button>
      </div>
    </header>

    <!-- sidebar -->
    <aside class="bg-sidebar/85 border-sidebar-border relative z-10 flex backdrop-blur-md flex-col justify-between border-r p-3">
      <nav class="relative flex flex-col gap-0.5">
        <div
          class="bg-sidebar-accent absolute inset-x-0 top-0 h-9 rounded-md transition-transform duration-300 ease-(--ease-out-expo)"
          style="transform: translateY({tab * 38}px)"
        ></div>
        {#each tabs as t, i}
          <button
            class="relative flex h-9 items-center gap-2.5 rounded-md px-3 text-sm font-medium transition-colors {tab === i ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => (tabByDevice[deviceId] = i)}
          >
            <Icon name={t.icon} size={17} />
            {t.label}
          </button>
        {/each}
      </nav>

      {#if mod?.Card}
        <mod.Card />
      {:else if mod}
        <div class="border-sidebar-border bg-card flex items-center gap-3 rounded-lg border p-3">
          <Icon name={mod.icon} size={22} class="text-muted-foreground" />
          <div class="flex min-w-0 flex-col">
            <span class="truncate text-sm font-medium">{displayName(mod)}</span>
            <span class="text-xs {dev.connected ? 'text-success' : 'text-warning'}">{dev.connected ? "Online" : "Procurando"}</span>
          </div>
        </div>
      {/if}
    </aside>

    <!-- content -->
    <main bind:this={mainEl} class="relative z-10 min-h-0 overflow-y-auto">
      {#if ready && Page}
        {#key deviceId + ":" + tab}
          <div class="mx-auto max-w-4xl px-8 py-7" in:fly={{ y: 8, duration: 280, easing: expoOut }}>
            <Page />
          </div>
        {/key}
      {:else if mod}
        <div class="flex h-full flex-col items-center justify-center gap-3 px-8 text-center" in:fade>
          <div class="relative grid size-16 place-items-center">
            <span class="border-brand/40 absolute inset-0 animate-ping rounded-full border"></span>
            <div class="bg-card border-border grid size-16 place-items-center rounded-full border"><Icon name={mod.icon} size={26} class="text-muted-foreground" /></div>
          </div>
          <h2 class="mt-3 text-lg font-semibold tracking-tight">{waiting.title}</h2>
          <p class="text-muted-foreground max-w-sm text-sm">{waiting.text}</p>
          {#if !dev.driver && !dev.reason}
            <code class="text-muted-foreground/70 mt-1 font-mono text-[11px]">sem driver "{mod.id}" no backend</code>
          {:else if dev.reason && !waiting.hideReason}
            <code class="text-muted-foreground/70 mt-1 font-mono text-[11px]">{dev.reason}</code>
          {/if}
        </div>
      {/if}
    </main>
  </div>
</Tooltip.Provider>

<Toaster position="bottom-right" />
