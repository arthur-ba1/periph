<script>
  // One tab of a package device: its settings, generated from device.json.
  // Plain values share a card (one row each); arrays get a card of their own.
  import * as Card from "$lib/components/ui/card/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import PageHeader from "$lib/components/page-header.svelte";
  import SettingRow from "$lib/components/setting-row.svelte";
  import { devices } from "$lib/device-state.svelte.js";
  import Control from "./Control.svelte";
  import { text, settingOf } from "./module.js";

  let { pkg, page } = $props();

  let dev = $derived(devices[pkg.manifest.id]);
  let settings = $derived(page.settings.map((id) => settingOf(pkg, id)).filter(Boolean));
  let rows = $derived(settings.filter((s) => !s.count));
  let lists = $derived(settings.filter((s) => s.count));
</script>

{#if dev?.config}
  <PageHeader title={text(pkg, `pages.${page.id}`, page.id)} description={text(pkg, `pages.${page.id}Description`)} />

  <div class="grid gap-4">
    {#each lists as s (s.id)}
      <Card.Root>
        <Card.Header>
          <Card.Title>{text(pkg, `settings.${s.id}.label`, s.id)}</Card.Title>
          {#if text(pkg, `settings.${s.id}.description`)}<Card.Description>{text(pkg, `settings.${s.id}.description`)}</Card.Description>{/if}
        </Card.Header>
        <Card.Content>
          <Control {pkg} setting={s} {dev} />
        </Card.Content>
      </Card.Root>
    {/each}

    {#if rows.length}
      <Card.Root class="py-2">
        <Card.Content>
          {#each rows as s, i (s.id)}
            {#if i > 0}<Separator />{/if}
            <SettingRow title={text(pkg, `settings.${s.id}.label`, s.id)} description={text(pkg, `settings.${s.id}.description`)}>
              <Control {pkg} setting={s} {dev} />
            </SettingRow>
          {/each}
        </Card.Content>
      </Card.Root>
    {/if}
  </div>
{/if}
