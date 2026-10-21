<script lang="ts">
  import type { IconName } from "../lib/icons";
  import { store, type ToastTone } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  const TONE_ICON: Record<ToastTone, IconName> = {
    info: "info",
    success: "check",
    warn: "alert",
    error: "alert",
  };
</script>

<div class="toasts" aria-live="polite" aria-atomic="false">
  {#each store.toasts as toast (toast.id)}
    <div class="toast tone-{toast.tone}" role="status">
      <span class="toast-icon"><Icon name={TONE_ICON[toast.tone]} size={16} /></span>
      <span class="toast-msg">{toast.message}</span>
      {#if toast.action}
        <button
          type="button"
          class="toast-action"
          onclick={() => {
            toast.action?.run();
            store.dismissToast(toast.id);
          }}>{toast.action.label}</button
        >
      {/if}
      <button
        type="button"
        class="toast-close"
        aria-label="关闭提示"
        onclick={() => store.dismissToast(toast.id)}
      >
        <Icon name="x" size={13} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    bottom: 20px;
    left: 50%;
    z-index: 100;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: min(92vw, 460px);
    transform: translateX(-50%);
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 100%;
    padding: 9px 8px 9px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-2);
    box-shadow: var(--shadow-toast);
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.45;
    pointer-events: auto;
    animation: rise-in 180ms var(--ease);
  }

  .toast-icon {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    color: var(--text-2);
  }

  .tone-success .toast-icon {
    color: var(--success);
  }
  .tone-error .toast-icon {
    color: var(--danger);
  }
  .tone-warn .toast-icon {
    color: var(--warn);
  }
  .tone-info .toast-icon {
    color: var(--primary);
  }

  .toast-msg {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .toast-action {
    flex: 0 0 auto;
    padding: 3px 7px;
    border-radius: var(--r-xs);
    color: var(--primary);
    font-size: 12px;
    font-weight: 600;
  }

  .toast-action:hover {
    background: var(--primary-soft);
  }

  .toast-close {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: var(--r-xs);
    color: var(--text-3);
  }

  .toast-close:hover {
    background: var(--hover);
    color: var(--text);
  }

  @media (max-width: 639px) {
    .toasts {
      bottom: calc(76px + env(safe-area-inset-bottom));
    }
  }
</style>
