<script>
  let { title, description, children } = $props();
</script>

<div class="flex items-center justify-between gap-6 py-4">
  <div class="min-w-0">
    <div class="text-sm font-medium">{title}</div>
    {#if description}<div class="text-muted-foreground mt-0.5 text-[13px] leading-snug">{description}</div>{/if}
  </div>
  <div class="shrink-0">{@render children?.()}</div>
</div>
