<script lang="ts">
  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    value: string;
    options: ReadonlyArray<Option>;
    label: string;
    disabled?: boolean;
    onchange?: (value: string) => void;
  }

  let { value, options, label, disabled = false, onchange }: Props = $props();
</script>

<div class="seg" role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      class="seg-btn"
      aria-pressed={option.value === value}
      {disabled}
      onclick={() => onchange?.(option.value)}>{option.label}</button
    >
  {/each}
</div>
