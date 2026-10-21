<script lang="ts">
  import { ICONS, type IconName } from "../lib/icons";

  interface Props {
    name: IconName;
    size?: number;
    stroke?: number;
    class?: string;
  }

  let { name, size = 18, stroke = 1.5, class: className = "" }: Props = $props();
</script>

<svg
  class={className}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#each ICONS[name] as shape}
    {#if shape.tag === "path"}
      <path d={shape.d} />
    {:else if shape.tag === "circle"}
      <circle cx={shape.cx} cy={shape.cy} r={shape.r} />
    {:else}
      <rect x={shape.x} y={shape.y} width={shape.width} height={shape.height} rx={shape.rx} />
    {/if}
  {/each}
</svg>
