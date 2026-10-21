<script lang="ts">
  import { onMount } from "svelte";
  import { isMock, platform } from "./lib/api";
  import { store } from "./lib/state.svelte";
  import BrandMark from "./components/BrandMark.svelte";
  import DownloadTab from "./components/DownloadTab.svelte";
  import Icon from "./components/Icon.svelte";
  import SettingsDrawer from "./components/SettingsDrawer.svelte";
  import TabBar from "./components/TabBar.svelte";
  import ToastHost from "./components/ToastHost.svelte";
  import TopBar from "./components/TopBar.svelte";
  import TransferTab from "./components/TransfersTab.svelte";
  import UploadTab from "./components/UploadTab.svelte";
  import WelcomePage from "./components/WelcomePage.svelte";

  let nativeCleanup: (() => void) | null = null;
  let mediaCleanup: (() => void) | null = null;
  let destroyed = false;

  // 主题：跟随 config / 预览 override，写入 <html data-theme>
  $effect(() => {
    store.applyTheme();
  });

  async function setupNativeDragDrop(): Promise<(() => void) | null> {
    if (isMock || platform() === "web") return null;
    try {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      return await getCurrentWebview().onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          store.dragActive = true;
        } else if (payload.type === "drop") {
          store.dragActive = false;
          void store.uploadPaths(payload.paths);
        } else {
          store.dragActive = false;
        }
      });
    } catch (error) {
      console.warn("[互联] onDragDropEvent 不可用，回退到 window 事件", error);
      return setupWindowDragDrop();
    }
  }

  function setupWindowDragDrop(): (() => void) | null {
    const handler = (event: Event): void => {
      const detail =
        event instanceof CustomEvent
          ? (event.detail as { paths?: string[] } | undefined)
          : undefined;
      store.dragActive = false;
      const paths = detail?.paths;
      if (paths && paths.length) void store.uploadPaths(paths);
    };
    window.addEventListener("tauri://drag-drop", handler as EventListener);
    return () => window.removeEventListener("tauri://drag-drop", handler as EventListener);
  }

  onMount(() => {
    void store.boot();

    void setupNativeDragDrop().then((cleanup) => {
      if (destroyed) cleanup?.();
      else nativeCleanup = cleanup;
    });

    const media = window.matchMedia("(prefers-color-scheme: light)");
    const onSystemThemeChange = (): void => {
      if (store.effectiveTheme === "system") store.applyTheme();
    };
    media.addEventListener("change", onSystemThemeChange);
    mediaCleanup = () => media.removeEventListener("change", onSystemThemeChange);

    return () => {
      destroyed = true;
      nativeCleanup?.();
      mediaCleanup?.();
      store.dispose();
    };
  });
</script>

<div class="app">
  {#if store.booting}
    <div class="boot">
      <BrandMark size={44} />
      <div class="boot-name">互联</div>
      <div class="boot-status"><span class="spinner"></span>正在启动…</div>
    </div>
  {:else if store.bootError}
    <div class="boot">
      <span class="boot-icon"><Icon name="alert" size={22} /></span>
      <div class="boot-name">启动失败</div>
      <p class="boot-msg">{store.bootError}</p>
      <button type="button" class="btn btn-primary" onclick={() => store.boot()}>重试</button>
      {#if isMock}
        <p class="boot-hint">Mock 模式：请确认 dev:mock 服务运行正常。</p>
      {/if}
    </div>
  {:else if !store.connected}
    <WelcomePage />
  {:else}
    <TopBar />
    <div class="stage">
      <div class="stage-inner">
        <TabBar />
        <main class="content">
          {#if store.tab === "upload"}
            <UploadTab />
          {:else if store.tab === "download"}
            <DownloadTab />
          {:else}
            <TransferTab />
          {/if}
        </main>
      </div>
    </div>
  {/if}

  <SettingsDrawer />
  <ToastHost />

  {#if store.dragActive}
    <div class="drag-overlay" aria-hidden="true">
      <div class="drag-card">
        <span class="drag-icon"><Icon name="upload" size={26} /></span>
        <span class="drag-title">松开，上传到「{store.transferFolderName}」</span>
        <span class="drag-sub">文件只会保存到你的阿里云盘</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    height: 100dvh;
    overflow: hidden;
    background: var(--bg);
  }

  .stage {
    flex: 1 1 auto;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
  }

  .stage-inner {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    max-width: 680px;
    margin: 0 auto;
    padding: 18px 24px 36px;
  }

  .content {
    min-width: 0;
  }

  .boot {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 32px;
    text-align: center;
  }

  .boot-name {
    margin-top: 4px;
    color: var(--text);
    font-size: 17px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-indent: 0.12em;
  }

  .boot-status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-3);
    font-size: 12.5px;
  }

  .boot-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: var(--r-xl);
    background: var(--danger-soft);
    color: var(--danger);
  }

  .boot-msg {
    max-width: 380px;
    margin: 0;
    color: var(--text-3);
    font-size: 12.5px;
    line-height: 1.6;
  }

  .boot-hint {
    margin: 4px 0 0;
    color: var(--text-3);
    font-size: 11.5px;
  }

  .drag-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--bg) 72%, transparent);
    backdrop-filter: blur(3px);
    -webkit-backdrop-filter: blur(3px);
    pointer-events: none;
    animation: fade-in 120ms var(--ease);
  }

  .drag-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 34px 52px;
    border: 1.5px dashed var(--primary);
    border-radius: var(--r-2xl);
    background: var(--surface);
    box-shadow: var(--shadow-drawer);
    text-align: center;
  }

  .drag-icon {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: var(--r-xl);
    background: var(--primary-soft);
    color: var(--primary);
  }

  .drag-title {
    color: var(--text);
    font-size: 14.5px;
    font-weight: 500;
  }

  .drag-sub {
    color: var(--text-3);
    font-size: 12px;
  }

  @media (max-width: 639px) {
    .stage-inner {
      gap: 14px;
      padding: 14px 16px calc(86px + env(safe-area-inset-bottom));
    }

    .drag-card {
      padding: 26px 28px;
    }
  }
</style>
