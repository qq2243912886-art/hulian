<script lang="ts">
  import { onMount } from "svelte";
  import { formatBytes, timeAgo } from "../lib/format";
  import { store } from "../lib/state.svelte";
  import EmptyState from "./EmptyState.svelte";
  import FileGlyph from "./FileGlyph.svelte";
  import Icon from "./Icon.svelte";

  let selected = $state<string[]>([]);
  let dispatching = $state(false);

  const files = $derived(store.remoteFiles);
  const selectedFiles = $derived(files.filter((file) => selected.includes(file.fileId)));
  const selectedSize = $derived(selectedFiles.reduce((total, file) => total + file.size, 0));
  const allSelected = $derived(files.length > 0 && selectedFiles.length === files.length);
  const showSkeleton = $derived(store.remoteLoading && files.length === 0);

  onMount(() => {
    if (!store.remoteLoadedAt) void store.loadRemote();
  });

  function toggle(fileId: string): void {
    selected = selected.includes(fileId)
      ? selected.filter((id) => id !== fileId)
      : [...selected, fileId];
  }

  function toggleAll(): void {
    selected = allSelected ? [] : files.map((file) => file.fileId);
  }

  async function refresh(): Promise<void> {
    await store.loadRemote();
  }

  async function downloadOne(fileId: string, name: string): Promise<void> {
    await store.download([{ fileId, name }]);
  }

  async function downloadSelected(): Promise<void> {
    if (!selectedFiles.length || dispatching) return;
    dispatching = true;
    try {
      await store.download(selectedFiles.map((file) => ({ fileId: file.fileId, name: file.name })));
      selected = [];
    } finally {
      dispatching = false;
    }
  }
</script>

<section class="download">
  <header class="dl-head">
    <div class="dl-title">
      <h2 class="section-label">文件互传</h2>
      <span class="dl-sub">
        {#if showSkeleton}
          正在读取云端目录…
        {:else if store.remoteLoadedAt}
          {files.length} 个文件 · 更新于 {timeAgo(store.remoteLoadedAt)}
        {:else}
          云端「{store.transferFolderName}」目录
        {/if}
      </span>
    </div>
    <button
      type="button"
      class="icon-btn"
      aria-label="刷新云端文件列表"
      title="刷新"
      disabled={store.remoteLoading}
      onclick={refresh}
    >
      <span class="refresh-icon" class:spinning={store.remoteLoading}>
        <Icon name="refresh" size={17} />
      </span>
    </button>
  </header>

  {#if selectedFiles.length}
    <div class="select-bar">
      <span class="select-count">
        已选 {selectedFiles.length} 项 · {formatBytes(selectedSize)}
      </span>
      <div class="select-actions">
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => (selected = [])}>
          取消
        </button>
        <button
          type="button"
          class="btn btn-primary btn-sm"
          disabled={dispatching}
          onclick={downloadSelected}
        >
          {#if dispatching}
            <span class="spinner"></span>
          {:else}
            <Icon name="download" size={14} />
          {/if}
          下载
        </button>
      </div>
    </div>
  {/if}

  {#if showSkeleton}
    <div class="card sk-list" aria-hidden="true">
      {#each [0, 1, 2] as row (row)}
        <div class="sk-row">
          <div class="skeleton sk-glyph"></div>
          <div class="sk-main">
            <div class="skeleton sk-name"></div>
            <div class="skeleton sk-meta"></div>
          </div>
          <div class="skeleton sk-btn"></div>
        </div>
      {/each}
    </div>
  {:else if store.remoteError}
    <div class="card">
      <EmptyState icon="alert" title="云端目录读取失败" desc={store.remoteError}>
        <button type="button" class="btn btn-secondary" onclick={refresh}>重试</button>
      </EmptyState>
    </div>
  {:else if files.length === 0}
    <div class="card">
      <EmptyState
        icon="cloud"
        title="云端还没有文件"
        desc={`在「上传」里拖入第一个文件，它会被保存到云端「${store.transferFolderName}」目录。`}
      >
        <button type="button" class="btn btn-secondary" onclick={() => store.setTab("upload")}>
          去上传
        </button>
      </EmptyState>
    </div>
  {:else}
    <div class="card list">
      <div class="list-toolbar">
        <label class="select-all">
          <input
            type="checkbox"
            class="checkbox"
            checked={allSelected}
            aria-label="全选云端文件"
            onchange={toggleAll}
          />
          <span>全选</span>
        </label>
        <span class="list-count section-meta">{files.length} 项</span>
      </div>

      {#each files as file (file.fileId)}
        <div class="file-row" class:selected={selected.includes(file.fileId)}>
          <input
            type="checkbox"
            class="checkbox file-check"
            checked={selected.includes(file.fileId)}
            aria-label={`选择 ${file.name}`}
            onchange={() => toggle(file.fileId)}
          />
          <FileGlyph name={file.name} size={36} />
          <div class="file-main">
            <div class="file-name truncate" title={file.name}>{file.name}</div>
            <div class="file-meta num">
              {formatBytes(file.size)}
              <span class="dot">·</span>
              {timeAgo(file.createdAt)}
            </div>
          </div>
          <button
            type="button"
            class="icon-btn file-dl"
            aria-label={`下载 ${file.name}`}
            title="下载"
            onclick={() => downloadOne(file.fileId, file.name)}
          >
            <Icon name="download" size={17} />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .download {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .dl-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 2px;
  }

  .dl-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .dl-title .section-label {
    margin: 0;
    font-size: 13px;
  }

  .dl-sub {
    color: var(--text-3);
    font-size: 11.5px;
  }

  .refresh-icon {
    display: grid;
    place-items: center;
  }

  .refresh-icon.spinning {
    animation: spin 0.8s linear infinite;
  }

  .select-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px 8px 14px;
    border: 1px solid var(--primary-soft);
    border-radius: var(--r-md);
    background: var(--primary-soft);
    animation: rise-in 160ms var(--ease);
  }

  .select-count {
    color: var(--text);
    font-size: 12.5px;
    font-weight: 500;
  }

  .select-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
  }

  .list {
    overflow: hidden;
  }

  .list-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-faint);
    background: color-mix(in srgb, var(--surface-2) 60%, transparent);
  }

  .select-all {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    font-size: 12px;
    cursor: pointer;
    user-select: none;
  }

  .list-count {
    flex: 0 0 auto;
  }

  .file-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    transition: background-color var(--dur-2) var(--ease);
  }

  .file-row + .file-row {
    border-top: 1px solid var(--border-faint);
  }

  .file-row:hover {
    background: var(--hover);
  }

  .file-row.selected {
    background: color-mix(in srgb, var(--primary) 7%, transparent);
  }

  .file-check {
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease);
  }

  .file-row:hover .file-check,
  .file-row:focus-within .file-check,
  .file-row.selected .file-check {
    opacity: 1;
  }

  .file-main {
    flex: 1 1 auto;
    min-width: 0;
  }

  .file-name {
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .file-meta {
    margin-top: 1px;
    color: var(--text-3);
    font-size: 11.5px;
  }

  .dot {
    margin: 0 3px;
  }

  .file-dl {
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease);
  }

  .file-row:hover .file-dl,
  .file-row:focus-within .file-dl,
  .file-row.selected .file-dl {
    opacity: 1;
  }

  .sk-list {
    overflow: hidden;
  }

  .sk-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
  }

  .sk-row + .sk-row {
    border-top: 1px solid var(--border-faint);
  }

  .sk-glyph {
    flex: 0 0 auto;
    width: 36px;
    height: 36px;
    border-radius: var(--r-md);
  }

  .sk-main {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 7px;
  }

  .sk-name {
    width: 42%;
    height: 11px;
  }

  .sk-meta {
    width: 26%;
    height: 9px;
  }

  .sk-btn {
    flex: 0 0 auto;
    width: 28px;
    height: 28px;
    border-radius: 7px;
  }

  @media (max-width: 639px) {
    .file-check,
    .file-dl {
      opacity: 1;
    }

    .file-row {
      padding: 9px 10px;
      gap: 8px;
    }

    .file-dl {
      width: 34px;
      height: 34px;
    }

    .select-bar {
      flex-wrap: wrap;
      gap: 8px;
    }
  }
</style>
