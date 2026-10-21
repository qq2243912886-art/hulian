/**
 * 互联 · 全局应用状态（Svelte 5 runes）
 *
 * 唯一持有跨组件状态的地方：配置、云端列表、传输队列、toast、主题覆盖。
 * 组件只调用这里暴露的动作，后端交互一律经由 src/lib/api.ts。
 */
import {
  api,
  isMock,
  onTrayUpload,
  onTransferList,
  onTransferUpdate,
  platform,
} from "./api";
import type {
  AppConfig,
  AppInfo,
  DriveQuota,
  PdsAccount,
  RemoteFile,
  ThemeMode,
  TransferItem,
} from "./types";
import { shortName } from "./ui";

export type TabId = "upload" | "download" | "transfers";
export type ToastTone = "info" | "success" | "warn" | "error";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastItem {
  id: number;
  tone: ToastTone;
  message: string;
  action?: ToastAction;
}

export function errText(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "发生未知错误";
}

function sortTransfers(list: TransferItem[]): TransferItem[] {
  return [...list].sort((a, b) => b.createdAt - a.createdAt);
}

function cloneConfig(config: AppConfig): AppConfig {
  return { account: config.account ? { ...config.account } : null, settings: { ...config.settings } };
}

let toastSeq = 0;

class AppStore {
  booting = $state(true);
  bootError = $state<string | null>(null);

  config = $state<AppConfig | null>(null);
  appInfo = $state<AppInfo | null>(null);

  quota = $state<DriveQuota | null>(null);
  quotaLoading = $state(false);
  quotaError = $state<string | null>(null);

  transfers = $state<TransferItem[]>([]);

  remoteFiles = $state<RemoteFile[]>([]);
  remoteLoading = $state(false);
  remoteError = $state<string | null>(null);
  remoteLoadedAt = $state(0);

  tab = $state<TabId>("upload");

  settingsOpen = $state(false);
  settingsFocusAccount = $state(false);
  themeOverride = $state<ThemeMode | null>(null);

  toasts = $state<ToastItem[]>([]);
  dragActive = $state(false);

  private unsubs: Array<() => void> = [];
  private toastTimers = new Map<number, ReturnType<typeof setTimeout>>();
  private remoteRequest = 0;

  /* ------------------------------------------------------------- derived -- */

  get account(): PdsAccount | null {
    return this.config?.account ?? null;
  }

  get settings() {
    return this.config?.settings ?? null;
  }

  get connected(): boolean {
    return Boolean(this.config?.account);
  }

  get transferFolderName(): string {
    return this.settings?.transferFolderName || "文件互传";
  }

  get isDesktop(): boolean {
    if (this.appInfo) return this.appInfo.isDesktop;
    const p = platform();
    return p === "macos" || p === "windows" || p === "linux";
  }

  get effectiveTheme(): ThemeMode {
    return this.themeOverride ?? this.config?.settings.theme ?? "dark";
  }

  get activeTransfers(): TransferItem[] {
    return this.transfers
      .filter((t) => t.state === "running" || t.state === "queued")
      .sort((a, b) => b.createdAt - a.createdAt);
  }

  get historyTransfers(): TransferItem[] {
    return this.transfers
      .filter((t) => t.state !== "running" && t.state !== "queued")
      .sort((a, b) => (b.finishedAt ?? b.createdAt) - (a.finishedAt ?? a.createdAt));
  }

  get activeCount(): number {
    return this.transfers.filter((t) => t.state === "running" || t.state === "queued").length;
  }

  get historyCount(): number {
    return this.transfers.filter((t) => t.state !== "running" && t.state !== "queued").length;
  }

  get uploadQueue(): TransferItem[] {
    return this.transfers.filter(
      (t) => t.kind === "upload" && (t.state === "queued" || t.state === "running")
    );
  }

  /* ---------------------------------------------------------------- boot -- */

  async boot(): Promise<void> {
    this.booting = true;
    this.bootError = null;
    try {
      const [info, config] = await Promise.all([api.getAppInfo(), api.getConfig()]);
      this.appInfo = info;
      this.config = config;
    } catch (error) {
      this.bootError = errText(error);
      this.booting = false;
      return;
    }

    try {
      await this.bindEvents();
      this.transfers = sortTransfers(await api.listTransfers());
    } catch (error) {
      this.showToast("warn", `读取传输记录失败：${errText(error)}`);
    }

    this.booting = false;
    if (this.connected) void this.loadQuota();
    if (this.connected && this.tab === "download") void this.loadRemote();
  }

  private async bindEvents(): Promise<void> {
    const unUpdate = await onTransferUpdate((item) => this.applyTransferUpdate(item));
    const unList = await onTransferList((items) => {
      this.transfers = sortTransfers(items);
    });
    const unTray = await onTrayUpload(() => {
      void this.handleTrayUpload();
    });
    this.unsubs.push(unUpdate, unList, unTray);
  }

  /**
   * 托盘「快速上传」：Rust 端已经弹出文件选择器并开始上传，
   * 前端只需刷新队列并给出反馈，不能再次触发 pickAndUpload。
   */
  private async handleTrayUpload(): Promise<void> {
    this.showToast("info", "已从托盘加入上传");
    await this.refreshTransfers();
    this.setTab("transfers");
  }

  dispose(): void {
    for (const un of this.unsubs) {
      try {
        un();
      } catch {
        /* 忽略卸载异常 */
      }
    }
    this.unsubs = [];
    for (const timer of this.toastTimers.values()) clearTimeout(timer);
    this.toastTimers.clear();
  }

  /* ------------------------------------------------------------- transfer -- */

  private applyTransferUpdate(next: TransferItem): void {
    const index = this.transfers.findIndex((t) => t.id === next.id);
    const prev = index >= 0 ? this.transfers[index] : null;

    if (index >= 0) {
      const copy = [...this.transfers];
      copy[index] = next;
      this.transfers = copy;
    } else {
      this.transfers = sortTransfers([next, ...this.transfers]);
    }

    if (!prev || prev.state === next.state) return;

    if (next.state === "success") {
      this.showToast(
        "success",
        `「${shortName(next.name)}」${next.kind === "upload" ? "上传" : "下载"}完成`,
        { label: "查看", run: () => this.setTab("transfers") }
      );
    } else if (next.state === "error") {
      this.showToast(
        "error",
        `「${shortName(next.name)}」传输失败${next.error ? `：${next.error}` : ""}`
      );
    }
  }

  async refreshTransfers(): Promise<void> {
    try {
      this.transfers = sortTransfers(await api.listTransfers());
    } catch {
      /* 静默：事件流仍是主通道 */
    }
  }

  async pickAndUpload(): Promise<void> {
    try {
      await api.pickAndUpload();
      await this.refreshTransfers();
    } catch (error) {
      this.showToast("error", `无法选择文件：${errText(error)}`);
    }
  }

  /** 桌面端原生拖拽（Tauri drag-drop）落点 */
  async uploadPaths(paths: string[]): Promise<void> {
    this.dragActive = false;
    if (!paths.length) return;
    try {
      await api.uploadDropped(paths);
      this.showToast("info", `已加入上传队列 · ${paths.length} 个文件`);
      await this.refreshTransfers();
    } catch (error) {
      this.showToast("error", `上传失败：${errText(error)}`);
    }
  }

  /** 浏览器 mock 预览用：把 HTML5 拖入的文件模拟成路径 */
  async dropMockFiles(files: File[]): Promise<void> {
    this.dragActive = false;
    if (!isMock || !files.length) return;
    const paths = files.map((file) => `/mock-drop/${file.name}`);
    this.showToast("info", `Mock 预览：已模拟拖入 ${files.length} 个文件`);
    await this.uploadPaths(paths);
  }

  async download(files: { fileId: string; name: string }[]): Promise<void> {
    if (!files.length) return;
    try {
      await api.downloadFiles(files);
      await this.refreshTransfers();
      this.showToast(
        "success",
        files.length === 1
          ? `已开始下载「${shortName(files[0].name)}」`
          : `已开始下载 ${files.length} 个文件`,
        { label: "查看", run: () => this.setTab("transfers") }
      );
    } catch (error) {
      this.showToast("error", `下载失败：${errText(error)}`);
    }
  }

  async cancelTransfer(id: string): Promise<void> {
    try {
      await api.cancelTransfer(id);
    } catch (error) {
      this.showToast("error", `取消失败：${errText(error)}`);
    }
  }

  async retryTransfer(id: string): Promise<void> {
    try {
      await api.retryTransfer(id);
      await this.refreshTransfers();
    } catch (error) {
      this.showToast("error", `重试失败：${errText(error)}`);
    }
  }

  async removeTransfer(id: string): Promise<void> {
    const backup = this.transfers;
    this.transfers = this.transfers.filter((t) => t.id !== id);
    try {
      await api.removeTransfer(id);
    } catch (error) {
      this.transfers = backup;
      this.showToast("error", `移除失败：${errText(error)}`);
    }
  }

  async clearFinishedTransfers(): Promise<void> {
    const count = this.historyCount;
    if (!count) return;
    const backup = this.transfers;
    this.transfers = this.transfers.filter((t) => t.state === "running" || t.state === "queued");
    try {
      await api.clearFinishedTransfers();
      this.showToast("info", `已清空 ${count} 条记录`);
    } catch (error) {
      this.transfers = backup;
      this.showToast("error", `清空失败：${errText(error)}`);
    }
  }

  async openPath(path: string): Promise<void> {
    if (!path) return;
    try {
      await api.openPath(path);
    } catch (error) {
      this.showToast("error", `无法打开文件：${errText(error)}`);
    }
  }

  async revealInFolder(path: string): Promise<void> {
    if (!path) return;
    try {
      await api.revealInFolder(path);
    } catch (error) {
      this.showToast("error", `无法在文件夹中显示：${errText(error)}`);
    }
  }

  async openDownloadDir(): Promise<void> {
    try {
      await api.openDownloadDir();
    } catch (error) {
      this.showToast("error", `无法打开下载目录：${errText(error)}`);
    }
  }

  /* ----------------------------------------------------------------- cloud -- */

  async loadQuota(): Promise<void> {
    if (!this.connected) {
      this.quota = null;
      this.quotaError = null;
      return;
    }
    this.quotaLoading = true;
    this.quotaError = null;
    try {
      this.quota = await api.getDriveQuota();
    } catch (error) {
      this.quota = null;
      this.quotaError = errText(error);
    } finally {
      this.quotaLoading = false;
    }
  }

  async loadRemote(): Promise<void> {
    if (!this.connected) return;
    const request = ++this.remoteRequest;
    this.remoteLoading = true;
    this.remoteError = null;
    try {
      const files = await api.listRemote();
      if (request !== this.remoteRequest) return;
      this.remoteFiles = files.filter((file) => !file.isFolder);
      this.remoteLoadedAt = Date.now();
    } catch (error) {
      if (request === this.remoteRequest) this.remoteError = errText(error);
    } finally {
      if (request === this.remoteRequest) this.remoteLoading = false;
    }
  }

  /* --------------------------------------------------------------- config -- */

  async saveConfig(next: AppConfig, successMessage: string | null = "设置已保存"): Promise<boolean> {
    const prev = this.config;
    try {
      const saved = await api.saveConfig(cloneConfig(next));
      this.config = saved;
      this.themeOverride = null;

      if (saved.account) {
        void this.loadQuota();
        if (this.remoteLoadedAt) void this.loadRemote();
      } else {
        this.quota = null;
        this.quotaError = null;
        this.remoteFiles = [];
        this.remoteLoadedAt = 0;
        this.tab = "upload";
      }

      if (
        this.appInfo?.isDesktop &&
        prev &&
        saved.settings.autostart !== prev.settings.autostart
      ) {
        try {
          await api.setAutostart(saved.settings.autostart);
        } catch (error) {
          this.showToast("warn", `开机自启设置失败：${errText(error)}`);
        }
      }

      if (successMessage) this.showToast("success", successMessage);
      return true;
    } catch (error) {
      this.showToast("error", `保存失败：${errText(error)}`);
      return false;
    }
  }

  async importCliAccount(): Promise<boolean> {
    const config = this.config;
    if (!config) return false;
    try {
      const result = await api.importLocalCliConfig();
      if (!result.ok || !result.account) {
        this.showToast("error", result.message || "未找到可导入的 aliyun CLI 配置");
        return false;
      }
      const saved = await api.saveConfig({
        account: result.account,
        settings: { ...config.settings },
      });
      this.config = saved;
      this.themeOverride = null;
      void this.loadQuota();
      this.showToast(
        "success",
        `已连接云盘${result.account.userName ? `：${result.account.userName}` : ""}`
      );
      return true;
    } catch (error) {
      this.showToast("error", `导入失败：${errText(error)}`);
      return false;
    }
  }

  async signOut(): Promise<boolean> {
    const config = this.config;
    if (!config) return false;
    const ok = await this.saveConfig(
      { account: null, settings: { ...config.settings } },
      null
    );
    if (ok) {
      this.closeSettings();
      this.showToast("info", "已退出账号，文件不会自动同步");
    }
    return ok;
  }

  /* ------------------------------------------------------------------ ui -- */

  setTab(tab: TabId): void {
    if (this.tab === tab) return;
    this.tab = tab;
    if (tab === "download" && this.connected && !this.remoteLoadedAt && !this.remoteLoading) {
      void this.loadRemote();
    }
    if (tab === "transfers") void this.refreshTransfers();
  }

  openSettings(focusAccount = false): void {
    this.settingsFocusAccount = focusAccount;
    this.settingsOpen = true;
  }

  closeSettings(): void {
    this.settingsOpen = false;
    this.settingsFocusAccount = false;
    this.themeOverride = null;
  }

  previewTheme(theme: ThemeMode): void {
    this.themeOverride = theme;
  }

  applyTheme(): void {
    if (typeof document === "undefined") return;
    const mode = this.effectiveTheme;
    const resolved: "light" | "dark" =
      mode === "system"
        ? window.matchMedia("(prefers-color-scheme: light)").matches
          ? "light"
          : "dark"
        : mode;
    const root = document.documentElement;
    root.dataset.theme = resolved;
    root.style.colorScheme = resolved;
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute("content", resolved === "light" ? "#f4f5f7" : "#0b0d10");
  }

  showToast(tone: ToastTone, message: string, action?: ToastAction): void {
    const id = ++toastSeq;
    this.toasts = [...this.toasts, { id, tone, message, action }].slice(-3);
    const timer = setTimeout(() => this.dismissToast(id), tone === "error" ? 5200 : 3400);
    this.toastTimers.set(id, timer);
  }

  dismissToast(id: number): void {
    const timer = this.toastTimers.get(id);
    if (timer !== undefined) {
      clearTimeout(timer);
      this.toastTimers.delete(id);
    }
    this.toasts = this.toasts.filter((item) => item.id !== id);
  }
}

export const store = new AppStore();
