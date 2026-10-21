<script lang="ts">
  interface Props {
    size?: number;
    class?: string;
  }

  let { size = 28, class: className = "" }: Props = $props();
</script>

<span class="brand-mark {className}" style="--mark-size: {size}px" aria-hidden="true">
  <svg
    width={Math.round(size * 0.64)}
    height={Math.round(size * 0.64)}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M5 9.5h9.5" />
    <path d="m11.5 6.5 3 3-3 3" />
    <path d="M19 14.5H9.5" />
    <path d="m12.5 11.5-3 3 3 3" />
  </svg>
</span>

<style>
  .brand-mark {
    display: inline-grid;
    flex: 0 0 auto;
    place-items: center;
    width: var(--mark-size);
    height: var(--mark-size);
    border-radius: calc(var(--mark-size) * 0.29);
    background: linear-gradient(145deg, #7aa4ff 0%, #5b8def 46%, #4270d8 100%);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.28), 0 4px 14px rgba(64, 110, 220, 0.28);
    color: #fff;
  }
</style>
