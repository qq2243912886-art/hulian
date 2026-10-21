<script lang="ts">
  import { isMock } from "../lib/api";
  import { formatBytes } from "../lib/format";
  import { store } from "../lib/state.svelte";
  import { percentOf, progressOf } from "../lib/ui";
  import Icon from "./Icon.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  let dragOver = $state(false);
  let picking = $state(false);

  const queue = $derived(store.uploadQueue);

  async function pick(): Promise<void> {
    if (picking) return;
    picking = true;
    try {
      await store.pickAndUpload();
    } finally {
      picking = false;
    }
  }

  function onDragOver(event: DragEvent): void {
    event.preventDefault();
    if (!isMock) return;
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    dragOver = true;
  }

  function onDragLeave(): void {
    dragOver = false;
  }

  function onDrop(event: DragEvent): void {
    event.preventDefault();
    dragOver = false;
    if (!isMock) return;
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length) void store.dropMockFiles(files);
  }
</script>

<section class="upload">
  <button
    type="button"
    class="dropzone"
    class:dragover={dragOver}
    aria-label={`选择文件上传，保存到云端「${store.transferFolderName}」目录`}
    onclick={pick}
    ondragover={onDragOver}
    ondragleave={onDragLeave}
    ondrop={onDrop}
  >
    <span class="dz-glow" aria-hidden="true"></span>
    <span class="dz-icon"><Icon name="cloud" size={26} /></span>
    <span class="dz-title">拖入文件，开始互传</span>
    <span class="dz-sub">
      上传到云端「{store.transferFolderName}」目录 · 仅手动触发，无后台同步
    </span>
    <span class="dz-cta">
      {#if picking}
        <span class="spinner"></span>
        正在准备…
      {:else}
        <Icon name="upload" size={15} />
        选择文件
      {/if}
    </span>
  </button>

  <section class="queue" aria-label="上传队列">
    <header class="queue-head">
      <span class="section-label">上传队列</span>
      {#if queue.length}
        <span class="section-meta">{queue.length} 个文件</span>
      {/if}
    </header>

    {#if queue.length}
      <div class="card queue-list">
        {#each queue as item (item.id)}
          <div class="queue-row">
            <span class="queue-tile"><Icon name="upload" size={15} /></span>
            <div class="queue-main">
              <div class="queue-line">
                <span class="queue-name truncate" title={item.name}>{item.name}</span>
                <span class="queue-meta num">
                  {formatBytes(item.transferred)} / {formatBytes(item.size)}
                </span>
              </div>
              <ProgressBar value={progressOf(item)} label={`${item.name} 上传进度`} />
            </div>
            <span class="queue-pct num">{percentOf(item)}%</span>
            <button
              type="button"
              class="icon-btn icon-btn-sm"
              aria-label={`取消上传 ${item.name}`}
              title="取消上传"
              onclick={() => store.cancelTransfer(item.id)}
            >
              <Icon name="x" size={15} />
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="queue-empty">
        <span class="queue-empty-icon"><Icon name="upload" size={16} /></span>
        <span>暂无待上传文件，选择或拖入文件后会在这里显示进度</span>
      </div>
    {/if}
  </section>
</section>

<style>
  .upload {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .dropzone {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    width: 100%;
    min-height: 224px;
    padding: 32px 24px;
    overflow: hidden;
    border: 1.5px dashed var(--dropzone-border);
    border-radius: var(--r-2xl);
    background: transparent;
    color: var(--text);
    text-align: center;
    transition: border-color var(--dur-2) var(--ease), background-color var(--dur-2) var(--ease);
  }

  .dropzone:hover {
    border-color: var(--border-strong);
    background: var(--hover);
  }

  .dropzone.dragover {
    border-color: var(--primary);
    background: var(--primary-soft);
  }

  .dz-glow {
    position: absolute;
    inset: 0;
    background: radial-gradient(60% 90% at 50% 0%, var(--primary-soft), transparent 70%);
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--dur-3) var(--ease);
  }

  .dropzone:hover .dz-glow,
  .dropzone.dragover .dz-glow {
    opacity: 1;
  }

  .dz-icon {
    position: relative;
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    margin-bottom: 6px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: var(--surface-2);
    color: var(--text-2);
    transition: background-color var(--dur-2) var(--ease), border-color var(--dur-2) var(--ease),
      color var(--dur-2) var(--ease);
  }

  .dropzone:hover .dz-icon,
  .dropzone.dragover .dz-icon {
    border-color: transparent;
    background: var(--primary-soft);
    color: var(--primary);
  }

  .dz-title {
    position: relative;
    color: var(--text);
    font-size: 15px;
    font-weight: 500;
    letter-spacing: 0.01em;
  }

  .dz-sub {
    position: relative;
    max-width: 380px;
    color: var(--text-3);
    font-size: 12.5px;
    line-height: 1.55;
  }

  .dz-cta {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    margin-top: 14px;
    padding: 0 14px;
    border-radius: 9px;
    background: var(--surface-3);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    transition: background-color var(--dur-2) var(--ease), color var(--dur-2) var(--ease);
  }

  .dropzone:hover .dz-cta,
  .dropzone.dragover .dz-cta {
    background: var(--primary);
    color: #fff;
  }

  .queue-head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 10px;
    padding: 0 2px;
  }

  .queue-list {
    overflow: hidden;
  }

  .queue-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 12px;
  }

  .queue-row + .queue-row {
    border-top: 1px solid var(--border-faint);
  }

  .queue-tile {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: var(--primary-soft);
    color: var(--primary);
  }

  .queue-main {
    flex: 1 1 auto;
    min-width: 0;
  }

  .queue-line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 6px;
  }

  .queue-name {
    min-width: 0;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 500;
  }

  .queue-meta {
    flex: 0 0 auto;
    color: var(--text-3);
    font-size: 11px;
  }

  .queue-pct {
    flex: 0 0 auto;
    min-width: 34px;
    color: var(--text-2);
    font-size: 11.5px;
    text-align: right;
  }

  .queue-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 15px 16px;
    border: 1px solid var(--border-faint);
    border-radius: var(--r-lg);
    background: var(--surface);
    color: var(--text-3);
    font-size: 12.5px;
  }

  .queue-empty-icon {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    color: var(--text-3);
  }

  @media (max-width: 639px) {
    .dropzone {
      min-height: 190px;
      padding: 26px 18px;
    }

    .dz-sub {
      font-size: 12px;
    }

    .upload {
      gap: 18px;
    }

    .queue-empty {
      padding: 14px 12px;
      text-align: left;
      line-height: 1.5;
    }
  }
</style>
