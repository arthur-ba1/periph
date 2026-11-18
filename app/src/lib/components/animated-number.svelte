<script>
  import { Tween } from "svelte/motion";
  import { expoOut } from "svelte/easing";

  let { value = 0, duration = 600, decimals = 0, class: className = "" } = $props();
  const t = new Tween(0, { duration, easing: expoOut });
  $effect(() => {
    t.target = value ?? 0;
  });
</script>

<span class="tabular {className}">{t.current.toFixed(decimals)}</span>
