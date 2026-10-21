<script lang="ts">
  import { formatBytes } from "../lib/format";
  import { store } from "../lib/state.svelte";
  import BrandMark from "./BrandMark.svelte";
  import Icon from "./Icon.svelte";

  const userName = $derived(store.account?.userName || store.account?.domainId || "已连接");
  const driveName = $derived(store.quota?.driveName || store.account?.driveName || "");
  const quotaText = $derived(
    store.quota ? `${formatBytes(store.quota.usedSize)} / ${formatBytes(store.quota.totalSize)}` : ""
  );
  const quotaLabel = $derived(
    quotaText
      ? `云盘状态：${userName}${driveName ? ` · ${driveName}` : ""} · 已用 ${quotaText}`
      : `云盘状态：${userName}`
  );
</script>

<header class="topbar">
  <div class="topbar-inner">
    <div class="brand">
      <BrandMark size={24} />
      <span class="brand-name">互联</span>
    </div>

    <span class="topbar-spacer"></span>

    <button
      type="button"
      class="status-pill"
      aria-label={`${quotaLabel}，点击打开设置`}
      title="云盘状态 · 点击打开设置"
      onclick={() => store.openSettings(true)}
    >
      <span class="status-dot" class:warn={Boolean(store.quotaError)}></span>
      <span class="pill-user">{userName}</span>
      {#if driveName}
        <span class="pill-sep">·</span>
        <span class="pill-drive">{driveName}</span>
      {/if}
      {#if quotaText}
        <span class="pill-sep pill-quota-sep">·</span>
        <span class="pill-quota num">{quotaText}</span>
      {:else if store.quotaLoading}
        <span class="pill-sep pill-quota-sep">·</span>
        <span class="pill-quota">读取容量…</span>
      {:else if store.quotaError}
        <span class="pill-sep pill-quota-sep">·</span>
        <span class="pill-quota">容量不可用</span>
      {/if}
    </button>

    <button
      type="button"
      class="icon-btn settings-btn"
      aria-label="打开设置"
      title="设置"
      onclick={() => store.openSettings()}
    >
      <Icon name="settings" size={18} />
    </button>
  </div>
</header>

<style>
  .topbar {
    flex: 0 0 auto;
    border-bottom: 1px solid var(--border);
    background: var(--bg-header);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
  }

  .topbar-inner {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    max-width: 680px;
    height: 54px;
    margin: 0 auto;
    padding: 0 24px;
  }

  .brand {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }

  .brand-name {
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.08em;
  }

  .topbar-spacer {
    flex: 1 1 auto;
  }

  .status-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--r-full);
    background: var(--surface);
    color: var(--text-2);
    font-size: 11.5px;
    transition: background-color var(--dur-2) var(--ease), border-color var(--dur-2) var(--ease),
      color var(--dur-2) var(--ease);
  }

  .status-pill:hover {
    border-color: var(--border-strong);
    background: var(--surface-2);
    color: var(--text);
  }

  .status-dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--success);
    box-shadow: 0 0 0 3px var(--success-soft);
  }

  .status-dot.warn {
    background: var(--warn);
    box-shadow: 0 0 0 3px var(--warn-soft);
  }

  .pill-user {
    overflow: hidden;
    max-width: 96px;
    color: var(--text);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pill-sep {
    color: var(--text-3);
  }

  .pill-drive {
    overflow: hidden;
    max-width: 96px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pill-quota {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .settings-btn {
    margin-left: 2px;
  }

  @media (max-width: 639px) {
    .topbar-inner {
      height: 48px;
      gap: 10px;
      padding: 0 16px;
    }

    .brand-name {
      font-size: 14px;
    }

    .pill-quota,
    .pill-quota-sep {
      display: none;
    }

    .pill-user,
    .pill-drive {
      max-width: 72px;
    }
  }

  @media (max-width: 479px) {
    .pill-sep,
    .pill-drive {
      display: none;
    }
  }
</style>
