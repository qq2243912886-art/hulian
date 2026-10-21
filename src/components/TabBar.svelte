<script lang="ts">
  import type { IconName } from "../lib/icons";
  import { store, type TabId } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  interface TabDef {
    id: TabId;
    label: string;
    icon: IconName;
  }

  const TABS: TabDef[] = [
    { id: "upload", label: "上传", icon: "upload" },
    { id: "download", label: "下载", icon: "download" },
    { id: "transfers", label: "传输", icon: "transfer" },
  ];
</script>

<nav class="tabs" aria-label="主要视图">
  {#each TABS as tab (tab.id)}
    <button
      type="button"
      class="tab"
      class:active={store.tab === tab.id}
      aria-current={store.tab === tab.id ? "page" : undefined}
      onclick={() => store.setTab(tab.id)}
    >
      <span class="tab-icon"><Icon name={tab.icon} size={16} stroke={1.6} /></span>
      <span class="tab-label">{tab.label}</span>
      {#if tab.id === "transfers" && store.activeCount > 0}
        <span class="tab-badge num" aria-label={`${store.activeCount} 个进行中任务`}>
          {store.activeCount}
        </span>
      {/if}
    </button>
  {/each}
</nav>

<style>
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    width: fit-content;
    padding: 3px;
    border: 1px solid var(--border-faint);
    border-radius: var(--r-md);
    background: var(--seg-bg);
  }

  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 7px;
    color: var(--text-2);
    font-size: 12.5px;
    transition: background-color var(--dur-2) var(--ease), color var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }

  .tab:hover {
    color: var(--text);
  }

  .tab.active {
    background: var(--seg-active);
    box-shadow: var(--seg-shadow);
    color: var(--text);
    font-weight: 500;
  }

  .tab-icon {
    display: grid;
    place-items: center;
  }

  .tab-badge {
    display: inline-grid;
    place-items: center;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--r-full);
    background: var(--primary-soft);
    color: var(--primary);
    font-size: 10px;
    font-weight: 600;
  }

  @media (max-width: 639px) {
    .tabs {
      position: fixed;
      right: 0;
      bottom: 0;
      left: 0;
      z-index: 30;
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 0;
      width: auto;
      padding: 6px 8px calc(6px + env(safe-area-inset-bottom));
      border: 0;
      border-top: 1px solid var(--border);
      border-radius: 0;
      background: var(--bg-header);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
    }

    .tab {
      flex-direction: column;
      justify-content: center;
      gap: 2px;
      height: 48px;
      background: transparent;
      box-shadow: none;
      color: var(--text-3);
      font-size: 10.5px;
    }

    .tab.active {
      background: transparent;
      box-shadow: none;
      color: var(--primary);
      font-weight: 500;
    }

    .tab-icon {
      height: 20px;
    }

    .tab-badge {
      position: absolute;
      top: 2px;
      left: calc(50% + 6px);
    }
  }
</style>
