<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    value: number;
    label: string;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    onchange?: (value: number) => void;
  }

  let {
    value,
    label,
    min = 1,
    max = 8,
    step = 1,
    disabled = false,
    onchange,
  }: Props = $props();

  function set(next: number): void {
    const clamped = Math.min(max, Math.max(min, next));
    if (clamped !== value) onchange?.(clamped);
  }
</script>

<div class="stepper" role="group" aria-label={label}>
  <button
    type="button"
    class="stepper-btn"
    aria-label={`减少${label}`}
    disabled={disabled || value <= min}
    onclick={() => set(value - step)}
  >
    <Icon name="minus" size={14} />
  </button>
  <span class="stepper-value" aria-live="polite">{value}</span>
  <button
    type="button"
    class="stepper-btn"
    aria-label={`增加${label}`}
    disabled={disabled || value >= max}
    onclick={() => set(value + step)}
  >
    <Icon name="plus" size={14} />
  </button>
</div>
