<script lang="ts">
  interface Props {
    value: number;
    tone?: "primary" | "success" | "danger";
    label?: string;
  }

  let { value, tone = "primary", label }: Props = $props();

  const percent = $derived(Math.round(Math.min(1, Math.max(0, Number.isFinite(value) ? value : 0)) * 100));
</script>

<div
  class="progress"
  role="progressbar"
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={percent}
  aria-label={label ?? "传输进度"}
>
  <div
    class="progress-fill"
    class:is-success={tone === "success"}
    class:is-danger={tone === "danger"}
    style="width: {percent}%"
  ></div>
</div>
