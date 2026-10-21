<script lang="ts">
  import { tick } from "svelte";
  import { api, platform } from "../lib/api";
  import { formatBytes } from "../lib/format";
  import { errText, store } from "../lib/state.svelte";
  import type {
    AppConfig,
    AppInfo,
    AppSettings,
    ConnectionStatus,
    PdsAccount,
    ThemeMode,
  } from "../lib/types";
  import BrandMark from "./BrandMark.svelte";
  import Icon from "./Icon.svelte";
  import Segmented from "./Segmented.svelte";
  import Stepper from "./Stepper.svelte";
  import Switch from "./Switch.svelte";

  interface Option {
    value: string;
    label: string;
  }

  const THEME_OPTIONS: Option[] = [
    { value: "dark", label: "深色" },
    { value: "light", label: "浅色" },
    { value: "system", label: "跟随系统" },
  ];

  const CHUNK_OPTIONS: Option[] = [
    { value: "4", label: "4 MB" },
    { value: "8", label: "8 MB" },
    { value: "16", label: "16 MB" },
    { value: "32", label: "32 MB" },
  ];

  const EMPTY_ACCOUNT: PdsAccount = { endpoint: "", domainId: "", driveId: "", accessToken: "" };
  const defaultSettings = (): AppSettings => ({
    transferFolderName: "文件互传",
    downloadDir: "",
    chunkSizeMb: 8,
    parallelParts: 1,
    parallelTransfers: 2,
    autostart: false,
    closeToTray: true,
    deviceName: "",
    theme: "dark",
  });

  const canImportCli = platform() !== "android" && platform() !== "ios";

  let account = $state<PdsAccount>({ ...EMPTY_ACCOUNT });
  let settings = $state<AppSettings>(defaultSettings());
  let originalAccount = $state<PdsAccount | null>(null);

  let showToken = $state(false);
  let saving = $state(false);
  let testing = $state(false);
  let importing = $state(false);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);
  let formError = $state<string | null>(null);
  let confirmSignOut = $state(false);

  let drawerEl = $state<HTMLElement | null>(null);
  let accountCardEl = $state<HTMLElement | null>(null);
  let endpointInput = $state<HTMLInputElement | null>(null);
  let closeBtn = $state<HTMLButtonElement | null>(null);

  // 每次打开时从已保存配置重建草稿
  $effect(() => {
    if (!store.settingsOpen) return;
    const config: AppConfig | null = store.config;
    if (!config) return;
    account = config.account ? { ...config.account } : { ...EMPTY_ACCOUNT };
    originalAccount = config.account ? { ...config.account } : null;
    settings = { ...config.settings };
    showToken = false;
    saving = false;
    testing = false;
    importing = false;
    testResult = null;
    formError = null;
    confirmSignOut = false;
  });

  // 打开时聚焦 / 关闭时归还焦点
  let wasOpen = false;
  let lastFocused: HTMLElement | null = null;

  $effect(() => {
    const open = store.settingsOpen;
    if (open && !wasOpen) {
      lastFocused =
        document.activeElement instanceof HTMLElement ? document.activeElement : null;
      const focusAccount = store.settingsFocusAccount;
      void tick().then(() => {
        if (focusAccount && endpointInput) {
          endpointInput.focus({ preventScroll: true });
          const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
          accountCardEl?.scrollIntoView({ block: "start", behavior: reduce ? "auto" : "smooth" });
        } else {
          closeBtn?.focus();
        }
      });
    } else if (!open && wasOpen) {
      lastFocused?.focus();
      lastFocused = null;
    }
    wasOpen = open;
  });

  // Esc 关闭 + Tab 焦点循环
  $effect(() => {
    if (!store.settingsOpen) return;
    const onKeydown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        event.preventDefault();
        cancel();
        return;
      }
      if (event.key !== "Tab" || !drawerEl) return;
      const focusables = Array.from(
        drawerEl.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )
      ).filter((el) => el.offsetParent !== null);
      if (!focusables.length) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  function platformLabel(value: AppInfo["platform"] | "web"): string {
    switch (value) {
      case "macos":
        return "macOS";
      case "windows":
        return "Windows";
      case "linux":
        return "Linux";
      case "android":
        return "Android";
      case "ios":
        return "iOS";
      default:
        return "Web 预览";
    }
  }

  function cancel(): void {
    if (saving) return;
    store.closeSettings();
  }

  function setTheme(value: string): void {
    const theme = value as ThemeMode;
    settings.theme = theme;
    store.previewTheme(theme);
  }

  interface AccountDraftResult {
    account: PdsAccount | null;
    error: string | null;
  }

  function normalizeAccount(): AccountDraftResult {
    const endpoint = account.endpoint.trim();
    const domainId = account.domainId.trim();
    const driveId = account.driveId.trim();
    const accessToken = account.accessToken.trim();
    const values = [endpoint, domainId, driveId, accessToken];
    const filled = values.filter((value) => value.length > 0).length;

    if (filled === 0) return { account: null, error: null };
    if (filled < 4) return { account: null, error: "账号四项凭据需要完整填写" };

    const next: PdsAccount = { endpoint, domainId, driveId, accessToken };
    const sameTarget =
      originalAccount !== null &&
      originalAccount.endpoint === endpoint &&
      originalAccount.domainId === domainId &&
      originalAccount.driveId === driveId;

    if (sameTarget && originalAccount) {
      if (originalAccount.userId) next.userId = originalAccount.userId;
      if (originalAccount.userName) next.userName = originalAccount.userName;
      if (originalAccount.driveName) next.driveName = originalAccount.driveName;
    } else {
      if (account.userId) next.userId = account.userId;
      if (account.userName) next.userName = account.userName;
      if (account.driveName) next.driveName = account.driveName;
    }
    return { account: next, error: null };
  }

  async function save(): Promise<void> {
    if (saving) return;
    formError = null;

    const { account: nextAccount, error } = normalizeAccount();
    if (error) {
      formError = error;
      return;
    }
    const folder = settings.transferFolderName.trim();
    if (!folder) {
      formError = "云端互传目录名不能为空";
      return;
    }
    const device = settings.deviceName.trim() || store.appInfo?.deviceName || "我的设备";

    const next: AppConfig = {
      account: nextAccount,
      settings: {
        ...settings,
        transferFolderName: folder,
        downloadDir: settings.downloadDir.trim(),
        deviceName: device,
        chunkSizeMb: Number(settings.chunkSizeMb),
        parallelParts: Number(settings.parallelParts),
        parallelTransfers: Number(settings.parallelTransfers),
      },
    };

    saving = true;
    const ok = await store.saveConfig(next);
    saving = false;
    if (ok) store.closeSettings();
  }

  async function testConnection(): Promise<void> {
    if (testing) return;
    formError = null;
    testResult = null;
    const { account: candidate, error } = normalizeAccount();
    if (error) {
      formError = error;
      return;
    }
    if (!candidate) {
      testResult = { ok: false, message: "请先填写账号凭据" };
      return;
    }
    testing = true;
    try {
      const result: ConnectionStatus = await api.testConnection(candidate);
      if (result.ok) {
        if (result.user) account.userName = result.user;
        if (result.drive) account.driveName = result.drive;
        const quota = result.quota
          ? ` · ${formatBytes(result.quota.usedSize)} / ${formatBytes(result.quota.totalSize)}`
          : "";
        testResult = {
          ok: true,
          message: `${result.message || "连接正常"}${result.user ? ` · ${result.user}` : ""}${quota}`,
        };
      } else {
        testResult = { ok: false, message: result.message || "连接失败，请检查凭据与 Endpoint" };
      }
    } catch (error) {
      testResult = { ok: false, message: errText(error) };
    } finally {
      testing = false;
    }
  }

  async function importAccount(): Promise<void> {
    if (importing) return;
    importing = true;
    testResult = null;
    formError = null;
    try {
      const result = await api.importLocalCliConfig();
      if (result.ok && result.account) {
        account = { ...result.account };
        testResult = {
          ok: true,
          message: `已读取本机配置${result.account.userName ? `：${result.account.userName}` : ""}，保存后生效`,
        };
      } else {
        testResult = { ok: false, message: result.message || "未找到可导入的 aliyun CLI 配置" };
      }
    } catch (error) {
      testResult = { ok: false, message: errText(error) };
    } finally {
      importing = false;
    }
  }

  async function signOut(): Promise<void> {
    if (!store.connected) return;
    if (!confirmSignOut) {
      confirmSignOut = true;
      return;
    }
    confirmSignOut = false;
    await store.signOut();
  }
</script>

{#if store.settingsOpen}
  <div class="drawer-layer">
    <button
      type="button"
      class="scrim"
      tabindex="-1"
      aria-label="关闭设置"
      onclick={cancel}
    ></button>

    <div
      class="drawer"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
      tabindex="-1"
      bind:this={drawerEl}
    >
      <header class="drawer-head">
        <h2 class="drawer-title" id="settings-title">设置</h2>
        <button
          type="button"
          class="icon-btn"
          aria-label="关闭设置"
          title="关闭"
          bind:this={closeBtn}
          onclick={cancel}
        >
          <Icon name="x" size={18} />
        </button>
      </header>

      <div class="drawer-body">
        <section class="section">
          <div class="section-head">
            <span class="section-label">账号</span>
            <span class="section-note">
              {store.connected ? "阿里云盘个人版 · PDS" : "未连接 · 填写后保存"}
            </span>
          </div>

          <div class="card section-card acct-card" bind:this={accountCardEl}>
            {#if store.connected}
              <div class="acct-status">
                <span class="acct-avatar" aria-hidden="true">
                  {store.account?.userName?.slice(0, 1) ?? "云"}
                </span>
                <div class="acct-info">
                  <div class="acct-name truncate">
                    {store.account?.userName || store.account?.domainId || "已连接"}
                  </div>
                  <div class="acct-meta truncate">
                    {store.quota?.driveName || store.account?.driveName || "云盘"}
                    {#if store.quota}
                      · {formatBytes(store.quota.usedSize)} / {formatBytes(store.quota.totalSize)}
                    {/if}
                  </div>
                </div>
                <span class="badge badge-success">
                  <Icon name="check" size={11} /> 已连接
                </span>
              </div>
            {/if}

            <div class="acct-body">
              <div class="form-grid">
                <div class="field field-wide">
                  <label class="field-label" for="acct-endpoint">Endpoint</label>
                  <input
                    id="acct-endpoint"
                    class="input input-mono"
                    bind:value={account.endpoint}
                    bind:this={endpointInput}
                    placeholder="bjxxxxx.api.aliyunfile.com"
                    autocomplete="off"
                    spellcheck="false"
                  />
                </div>
                <div class="field">
                  <label class="field-label" for="acct-domain">Domain ID</label>
                  <input
                    id="acct-domain"
                    class="input input-mono"
                    bind:value={account.domainId}
                    placeholder="bjxxxxx"
                    autocomplete="off"
                    spellcheck="false"
                  />
                </div>
                <div class="field">
                  <label class="field-label" for="acct-drive">Drive ID</label>
                  <input
                    id="acct-drive"
                    class="input input-mono"
                    bind:value={account.driveId}
                    placeholder="2"
                    autocomplete="off"
                    spellcheck="false"
                  />
                </div>
                <div class="field field-wide">
                  <label class="field-label" for="acct-token">Access Token</label>
                  <span class="token-wrap">
                    <input
                      id="acct-token"
                      class="input input-mono token-input"
                      type={showToken ? "text" : "password"}
                      bind:value={account.accessToken}
                      placeholder="uk-xxxxxxxxxxxxxxxx"
                      autocomplete="off"
                      spellcheck="false"
                    />
                    <button
                      type="button"
                      class="icon-btn icon-btn-sm token-eye"
                      aria-label={showToken ? "隐藏访问令牌" : "显示访问令牌"}
                      title={showToken ? "隐藏令牌" : "显示令牌"}
                      onclick={() => (showToken = !showToken)}
                    >
                      <Icon name={showToken ? "eyeOff" : "eye"} size={15} />
                    </button>
                  </span>
                </div>
              </div>

              <div class="acct-actions">
                {#if canImportCli}
                  <button
                    type="button"
                    class="btn btn-outline btn-sm"
                    disabled={importing}
                    onclick={importAccount}
                  >
                    {#if importing}
                      <span class="spinner"></span>
                    {:else}
                      <Icon name="terminal" size={14} />
                    {/if}
                    从 CLI 导入
                  </button>
                {/if}
                <button
                  type="button"
                  class="btn btn-secondary btn-sm"
                  disabled={testing}
                  onclick={testConnection}
                >
                  {#if testing}
                    <span class="spinner"></span>
                  {:else}
                    <Icon name="cloudCheck" size={14} />
                  {/if}
                  测试连接
                </button>
                {#if store.connected}
                  <span class="acct-spacer"></span>
                  {#if confirmSignOut}
                    <button
                      type="button"
                      class="btn btn-ghost btn-sm"
                      onclick={() => (confirmSignOut = false)}>取消</button
                    >
                    <button
                      type="button"
                      class="btn btn-danger btn-sm"
                      disabled={saving}
                      onclick={signOut}>确认退出</button
                    >
                  {:else}
                    <button type="button" class="btn btn-danger btn-sm" onclick={signOut}>
                      退出账号
                    </button>
                  {/if}
                {/if}
              </div>

              {#if testResult}
                <p class="test-result" class:ok={testResult.ok} role="status">
                  <Icon name={testResult.ok ? "check" : "alert"} size={14} />
                  <span>{testResult.message}</span>
                </p>
              {/if}

              <p class="acct-hint">
                <Icon name="shield" size={12} />
                凭据只保存在本机，直连你自己的阿里云盘
              </p>
            </div>
          </div>
        </section>

        <section class="section">
          <div class="section-head">
            <span class="section-label">传输</span>
            <span class="section-note">手动触发 · 无后台轮询</span>
          </div>
          <div class="card section-card">
            <div class="set-row">
              <div class="set-label">
                <label class="set-title" for="set-folder">云端互传目录</label>
                <span class="field-hint">根目录下自动创建</span>
              </div>
              <input
                id="set-folder"
                class="input set-input"
                bind:value={settings.transferFolderName}
                maxlength="40"
                placeholder="文件互传"
              />
            </div>

            <div class="set-row">
              <div class="set-label">
                <span class="set-title">下载目录</span>
                <span class="field-hint">文件保存位置</span>
              </div>
              <div class="set-inline">
                <span class="dir-path mono truncate" title={settings.downloadDir}>
                  {settings.downloadDir || "系统默认下载目录"}
                </span>
                <button
                  type="button"
                  class="btn btn-secondary btn-sm"
                  onclick={() => store.openDownloadDir()}
                >
                  <Icon name="folder" size={14} /> 打开
                </button>
              </div>
            </div>

            <div class="set-row">
              <div class="set-label">
                <span class="set-title">分片大小</span>
                <span class="field-hint">上传大文件时按此大小分片</span>
              </div>
              <Segmented
                label="分片大小"
                value={String(settings.chunkSizeMb)}
                options={CHUNK_OPTIONS}
                onchange={(value) => (settings.chunkSizeMb = Number(value))}
              />
            </div>

            <div class="set-row">
              <div class="set-label">
                <span class="set-title">并发分片</span>
                <span class="field-hint">建议 1（串行最稳），4 为实验性并发</span>
              </div>
              <Stepper
                label="并发分片数"
                min={1}
                max={4}
                value={settings.parallelParts}
                onchange={(value) => (settings.parallelParts = value)}
              />
            </div>

            <div class="set-row">
              <div class="set-label">
                <span class="set-title">同时任务数</span>
                <span class="field-hint">队列中同时进行的传输任务</span>
              </div>
              <Stepper
                label="同时任务数"
                min={1}
                max={4}
                value={settings.parallelTransfers}
                onchange={(value) => (settings.parallelTransfers = value)}
              />
            </div>
          </div>
        </section>

        <section class="section">
          <div class="section-head">
            <span class="section-label">外观</span>
          </div>
          <div class="card section-card">
            <div class="set-row">
              <div class="set-label">
                <span class="set-title">主题</span>
                <span class="field-hint">深色为默认，可跟随系统</span>
              </div>
              <Segmented
                label="主题"
                value={settings.theme}
                options={THEME_OPTIONS}
                onchange={setTheme}
              />
            </div>
          </div>
        </section>

        <section class="section">
          <div class="section-head">
            <span class="section-label">通用</span>
          </div>
          <div class="card section-card">
            <div class="set-row">
              <div class="set-label">
                <label class="set-title" for="set-device">设备名</label>
                <span class="field-hint">传输记录里标注来源</span>
              </div>
              <input
                id="set-device"
                class="input set-input"
                bind:value={settings.deviceName}
                maxlength="40"
                placeholder="我的设备"
              />
            </div>

            <div class="set-row">
              <div class="set-label">
                <span class="set-title">开机自启</span>
                {#if !store.isDesktop}<span class="tag">仅桌面端</span>{/if}
              </div>
              <Switch
                checked={settings.autostart}
                label="开机自启"
                disabled={!store.isDesktop}
                onchange={(value) => (settings.autostart = value)}
              />
            </div>

            <div class="set-row">
              <div class="set-label">
                <span class="set-title">关闭窗口时驻留托盘</span>
                <span class="field-hint">开启：关窗后驻留托盘；关闭：关窗即退出应用</span>
                {#if !store.isDesktop}<span class="tag">仅桌面端</span>{/if}
              </div>
              <Switch
                checked={settings.closeToTray}
                label="关闭窗口时驻留托盘"
                disabled={!store.isDesktop}
                onchange={(value) => (settings.closeToTray = value)}
              />
            </div>
          </div>
        </section>

        <div class="about">
          <BrandMark size={18} />
          <span>互联 {store.appInfo?.version ?? "0.1.0"}</span>
          <span class="about-sep">·</span>
          <span>{platformLabel(store.appInfo?.platform ?? "web")}</span>
          <span class="about-sep">·</span>
          <span>凭据仅保存在本机</span>
        </div>
      </div>

      <footer class="drawer-foot">
        {#if formError}
          <p class="form-error" role="alert">
            <Icon name="alert" size={14} />
            <span>{formError}</span>
          </p>
        {/if}
        <div class="foot-actions">
          <button type="button" class="btn btn-ghost" disabled={saving} onclick={cancel}>
            取消
          </button>
          <button type="button" class="btn btn-primary" disabled={saving} onclick={save}>
            {#if saving}<span class="spinner"></span>{/if}
            保存
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .drawer-layer {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    justify-content: flex-end;
  }

  .scrim {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background: var(--backdrop);
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
    border-radius: 0;
    animation: fade-in 180ms var(--ease);
  }

  .drawer {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    width: min(440px, 100%);
    height: 100%;
    border-left: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-drawer);
    outline: none;
    animation: drawer-in 200ms var(--ease-out);
  }

  @keyframes drawer-in {
    from {
      opacity: 0.5;
      transform: translateX(24px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .drawer-head {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: 54px;
    padding: 0 14px 0 20px;
    border-bottom: 1px solid var(--border);
  }

  .drawer-title {
    margin: 0;
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .drawer-body {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 22px;
    min-height: 0;
    padding: 18px 20px 24px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    padding: 0 2px;
  }

  .section-note {
    color: var(--text-3);
    font-size: 11px;
    text-align: right;
  }

  .section-card {
    overflow: hidden;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 12px;
    padding: 14px;
  }

  .field-wide {
    grid-column: 1 / -1;
  }

  .token-wrap {
    position: relative;
    display: block;
  }

  .token-input {
    padding-right: 38px;
  }

  .token-eye {
    position: absolute;
    top: 50%;
    right: 5px;
    transform: translateY(-50%);
  }

  .acct-status {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-faint);
  }

  .acct-avatar {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: var(--r-md);
    background: var(--primary-soft);
    color: var(--primary);
    font-size: 13px;
    font-weight: 600;
  }

  .acct-info {
    flex: 1 1 auto;
    min-width: 0;
  }

  .acct-name {
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .acct-meta {
    margin-top: 1px;
    color: var(--text-3);
    font-size: 11.5px;
  }

  .acct-body {
    padding: 14px;
  }

  .acct-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }

  .acct-spacer {
    flex: 1 1 auto;
  }

  .test-result {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 10px 0 0;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 12px;
    line-height: 1.5;
  }

  .test-result.ok {
    background: var(--success-soft);
    color: var(--success);
  }

  .test-result :global(svg) {
    flex: 0 0 auto;
    margin-top: 2px;
  }

  .acct-hint {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 10px 0 0;
    color: var(--text-3);
    font-size: 11px;
  }

  .set-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
  }

  .set-row + .set-row {
    border-top: 1px solid var(--border-faint);
  }

  .set-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .set-title {
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .set-input {
    flex: 0 1 190px;
    width: 190px;
    min-width: 0;
  }

  .set-inline {
    display: flex;
    flex: 0 1 auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .dir-path {
    max-width: 180px;
    color: var(--text-3);
    font-size: 11px;
  }

  .about {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 2px 2px 0;
    color: var(--text-3);
    font-size: 11px;
  }

  .about-sep {
    color: var(--text-3);
  }

  .drawer-foot {
    flex: 0 0 auto;
    padding: 12px 20px calc(12px + env(safe-area-inset-bottom));
    border-top: 1px solid var(--border);
    background: var(--surface);
  }

  .form-error {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0 0 10px;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 12px;
    line-height: 1.5;
  }

  .form-error :global(svg) {
    flex: 0 0 auto;
    margin-top: 2px;
  }

  .foot-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  @media (max-width: 639px) {
    .drawer {
      width: 100%;
      border-left: 0;
    }

    .drawer-head {
      height: 52px;
      padding-top: env(safe-area-inset-top);
    }

    .drawer-body {
      padding: 14px 16px 20px;
    }

    .set-row {
      flex-direction: column;
      align-items: stretch;
      gap: 8px;
    }

    .set-input {
      flex: 1 1 auto;
      width: 100%;
    }

    .set-inline {
      justify-content: space-between;
    }

    .dir-path {
      max-width: 62vw;
    }

    .drawer-foot {
      padding-right: 16px;
      padding-left: 16px;
    }

    .foot-actions .btn {
      flex: 1 1 auto;
    }
  }
</style>
