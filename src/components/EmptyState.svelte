<script lang="ts">
  import type { Snippet } from "svelte";
  import type { IconName } from "../lib/icons";
  import Icon from "./Icon.svelte";

  interface Props {
    icon: IconName;
    title: string;
    desc?: string | null;
    children?: Snippet;
  }

  let { icon, title, desc, children }: Props = $props();
</script>

<div class="empty">
  <span class="empty-icon"><Icon name={icon} size={22} /></span>
  <p class="empty-title">{title}</p>
  {#if desc}
    <p class="empty-desc">{desc}</p>
  {/if}
  {#if children}
    <div class="empty-actions">{@render children()}</div>
  {/if}
</div>
