<script lang="ts">
  import { formatBytes, formatSpeed, timeAgo } from "../lib/format";
  import { store } from "../lib/state.svelte";
  import type { TransferItem } from "../lib/types";
  import { kindLabel, percentOf, progressOf, transferStateMeta } from "../lib/ui";
  import Icon from "./Icon.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  interface Props {
    item: TransferItem;
  }

  let { item }: Props = $props();

  const meta = $derived(transferStateMeta(item));
  const percent = $derived(percentOf(item));
  const active = $derived(item.state === "running" || item.state === "queued");
</script>

<div class="t-row">
  <span class="t-tile" class:is-download={item.kind === "download"}>
    <Icon name={item.kind === "upload" ? "upload" : "download"} size={15} />
  </span>

  <div class="t-main">
    <div class="t-line1">
      <span class="t-name truncate" title={item.name}>{item.name}</span>
      <span class="badge badge-{meta.tone}">{meta.label}</span>
    </div>

    {#if active}
      <div class="t-progress">
        <ProgressBar value={progressOf(item)} label={`${item.name} ${kindLabel(item)}进度`} />
      </div>
      <div class="t-line2 num">
        <span>{formatBytes(item.transferred)} / {formatBytes(item.size)}</span>
        <span class="t-dot">·</span>
        <span>{formatSpeed(item.speedBps)}</span>
        <span class="t-pct">{percent}%</span>
      </div>
    {:else if item.state === "success"}
      <div class="t-line2 num">
        <span>{formatBytes(item.size)}</span>
        <span class="t-dot">·</span>
        <span>{item.finishedAt ? `${timeAgo(item.finishedAt)}完成` : `${timeAgo(item.createdAt)}完成`}</span>
      </div>
    {:else if item.state === "error"}
      <div class="t-line2 t-error">
        <Icon name="alert" size={13} />
        <span class="truncate">{item.error || `${kindLabel(item)}失败`}</span>
      </div>
    {:else}
      <div class="t-line2 num">
        <span>{formatBytes(item.transferred)} / {formatBytes(item.size)}</span>
        <span class="t-dot">·</span>
        <span>已取消</span>
      </div>
    {/if}
  </div>

  <div class="t-actions">
    {#if active}
      <button
        type="button"
        class="icon-btn icon-btn-sm"
        aria-label={`取消${kindLabel(item)} ${item.name}`}
        title="取消"
        onclick={() => store.cancelTransfer(item.id)}
      >
        <Icon name="x" size={15} />
      </button>
    {:else if item.state === "error" || item.state === "canceled"}
      <button
        type="button"
        class="icon-btn icon-btn-sm"
        aria-label={`重试${kindLabel(item)} ${item.name}`}
        title="重试"
        onclick={() => store.retryTransfer(item.id)}
      >
        <Icon name="refresh" size={15} />
      </button>
      <button
        type="button"
        class="icon-btn icon-btn-sm icon-btn-danger"
        aria-label={`移除记录 ${item.name}`}
        title="移除记录"
        onclick={() => store.removeTransfer(item.id)}
      >
        <Icon name="trash" size={15} />
      </button>
    {:else}
      <button
        type="button"
        class="icon-btn icon-btn-sm"
        aria-label={`打开 ${item.name}`}
        title="打开"
        onclick={() => store.openPath(item.localPath)}
      >
        <Icon name="external" size={15} />
      </button>
      <button
        type="button"
        class="icon-btn icon-btn-sm"
        aria-label={`在文件夹中显示 ${item.name}`}
        title="在文件夹中显示"
        onclick={() => store.revealInFolder(item.localPath)}
      >
        <Icon name="folderOpen" size={15} />
      </button>
      <button
        type="button"
        class="icon-btn icon-btn-sm icon-btn-danger"
        aria-label={`移除记录 ${item.name}`}
        title="移除记录"
        onclick={() => store.removeTransfer(item.id)}
      >
        <Icon name="trash" size={15} />
      </button>
    {/if}
  </div>
</div>

<style>
  .t-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 12px;
    border-bottom: 1px solid var(--border-faint);
  }

  .t-row:last-child {
    border-bottom: 0;
  }

  .t-tile {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--primary-soft);
    color: var(--primary);
  }

  .t-tile.is-download {
    background: var(--success-soft);
    color: var(--success);
  }

  .t-main {
    flex: 1 1 auto;
    min-width: 0;
  }

  .t-line1 {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .t-name {
    flex: 0 1 auto;
    min-width: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .t-progress {
    margin: 7px 0 5px;
  }

  .t-line2 {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    color: var(--text-3);
    font-size: 11.5px;
  }

  .t-line1 + .t-line2 {
    margin-top: 2px;
  }

  .t-dot {
    color: var(--text-3);
  }

  .t-pct {
    margin-left: auto;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }

  .t-error {
    margin-top: 3px;
    color: var(--danger);
  }

  .t-error :global(svg) {
    flex: 0 0 auto;
  }

  .t-actions {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 1px;
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease);
  }

  .t-row:hover .t-actions,
  .t-row:focus-within .t-actions {
    opacity: 1;
  }

  @media (max-width: 639px) {
    .t-actions {
      opacity: 1;
    }

    .t-row {
      padding: 10px;
      gap: 8px;
    }
  }
</style>
