<script>
  import { ICONS } from "./icons.js";
  let { name, size = 16, strokeWidth = 1.75, class: className = "", ...rest } = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class="shrink-0 {className}"
  aria-hidden="true"
  {...rest}
>
  {@html ICONS[name] ?? ""}
</svg>
