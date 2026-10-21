<script lang="ts">
  import { store } from "../lib/state.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Icon from "./Icon.svelte";
  import TransferRow from "./TransferRow.svelte";

  const active = $derived(store.activeTransfers);
  const history = $derived(store.historyTransfers);

  let clearing = $state(false);

  async function clearFinished(): Promise<void> {
    if (!history.length || clearing) return;
    clearing = true;
    try {
      await store.clearFinishedTransfers();
    } finally {
      clearing = false;
    }
  }
</script>

<section class="transfers">
  {#if active.length}
    <div class="group">
      <header class="group-head">
        <span class="section-label">进行中</span>
        <span class="group-count num">{active.length}</span>
      </header>
      <div class="card">
        {#each active as item (item.id)}
          <TransferRow {item} />
        {/each}
      </div>
    </div>
  {/if}

  {#if history.length}
    <div class="group">
      <header class="group-head">
        <span class="section-label">历史记录</span>
        <span class="group-count num">{history.length}</span>
      </header>
      <div class="card">
        {#each history as item (item.id)}
          <TransferRow {item} />
        {/each}
      </div>
      <div class="group-foot">
        <button
          type="button"
          class="btn btn-ghost btn-sm"
          disabled={clearing}
          onclick={clearFinished}
        >
          <Icon name="trash" size={14} />
          清空已完成（{history.length}）
        </button>
      </div>
    </div>
  {/if}

  {#if !active.length && !history.length}
    <div class="card">
      <EmptyState
        icon="transfer"
        title="还没有传输记录"
        desc="上传或下载文件后，进度与历史会显示在这里；关闭后重新打开也还在。"
      >
        <button type="button" class="btn btn-secondary" onclick={() => store.setTab("upload")}>
          上传文件
        </button>
        <button type="button" class="btn btn-ghost" onclick={() => store.setTab("download")}>
          下载文件
        </button>
      </EmptyState>
    </div>
  {/if}
</section>

<style>
  .transfers {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 0 2px;
  }

  .group-count {
    color: var(--text-3);
    font-size: 11.5px;
  }

  .group-foot {
    display: flex;
    justify-content: center;
    margin-top: 2px;
  }

  .card {
    overflow: hidden;
  }
</style>
