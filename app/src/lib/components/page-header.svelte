<script>
  let { title, description, children } = $props();
</script>

<div class="mb-6 flex items-end justify-between gap-4">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">{title}</h1>
    {#if description}<p class="text-muted-foreground mt-1 text-sm">{description}</p>{/if}
  </div>
  {@render children?.()}
</div>
