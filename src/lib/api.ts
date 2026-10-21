/**
 * 互联 · 前端 API 封装
 * mock 模式：`VITE_MOCK=1 pnpm dev` 可在纯浏览器里开发 UI（无需 Rust / 无真实云盘）。
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppConfig,
  AppInfo,
  ConnectionStatus,
  DriveQuota,
  ImportResult,
  PdsAccount,
  RemoteFile,
  TransferItem,
} from "./types";
import { EVENT_TRANSFER_LIST, EVENT_TRANSFER_UPDATE, EVENT_TRAY_UPLOAD } from "./types";
import * as mock from "./mock";

export const isMock = import.meta.env.VITE_MOCK === "1";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isMock || !inTauri) return mock.invokeMock<T>(cmd, args);
  return invoke<T>(cmd, args);
}

export const api = {
  getConfig: () => call<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => call<AppConfig>("save_config", { config }),
  importLocalCliConfig: () => call<ImportResult>("import_local_cli_config"),
  testConnection: (account?: PdsAccount) => call<ConnectionStatus>("test_connection", { account: account ?? null }),
  getDriveQuota: () => call<DriveQuota>("get_drive_quota"),
  /** 列出云端互传目录下的文件（created_at 倒序） */
  listRemote: () => call<RemoteFile[]>("list_remote"),
  /** 列出云端互传目录下的子目录 */
  listRemoteFolders: () => call<RemoteFile[]>("list_remote_folders"),
  listFolder: (parentFileId: string) => call<RemoteFile[]>("list_folder", { parentFileId }),
  /** 触发上传（本地绝对路径数组）；返回任务 id */
  uploadFiles: (paths: string[]) => call<string[]>("upload_files", { paths }),
  /** 打开系统文件选择器并上传（移动端也走这里） */
  pickAndUpload: () => call<string[]>("pick_and_upload"),
  /** 触发下载；返回任务 id */
  downloadFiles: (files: { fileId: string; name: string }[]) =>
    call<string[]>("download_files", { files }),
  listTransfers: () => call<TransferItem[]>("list_transfers"),
  cancelTransfer: (id: string) => call<void>("cancel_transfer", { id }),
  retryTransfer: (id: string) => call<string>("retry_transfer", { id }),
  removeTransfer: (id: string) => call<void>("remove_transfer", { id }),
  clearFinishedTransfers: () => call<void>("clear_finished_transfers"),
  openPath: (path: string) => call<void>("open_path", { path }),
  revealInFolder: (path: string) => call<void>("reveal_in_folder", { path }),
  openDownloadDir: () => call<void>("open_download_dir"),
  getAppInfo: () => call<AppInfo>("get_app_info"),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  hideMainWindow: () => call<void>("hide_main_window"),
  showMainWindow: () => call<void>("show_main_window"),
  quitApp: () => call<void>("quit_app"),
  /** 桌面端：把拖拽进来的文件路径上传（Tauri 拖拽事件由 UI 捕获后传路径） */
  uploadDropped: (paths: string[]) => call<string[]>("upload_files", { paths }),
};

/** 传输任务增量更新订阅 */
export async function onTransferUpdate(cb: (item: TransferItem) => void): Promise<UnlistenFn> {
  if (isMock) return mock.onMockTransferUpdate(cb);
  return listen<TransferItem>(EVENT_TRANSFER_UPDATE, (e) => cb(e.payload));
}

/** 传输任务列表整体刷新（重开窗口时拉全量） */
export async function onTransferList(cb: (items: TransferItem[]) => void): Promise<UnlistenFn> {
  if (isMock) return mock.onMockTransferList(cb);
  return listen<TransferItem[]>(EVENT_TRANSFER_LIST, (e) => cb(e.payload));
}

/** 托盘「快速上传」菜单触发 */
export async function onTrayUpload(cb: () => void): Promise<UnlistenFn> {
  if (isMock) return async () => {};
  return listen(EVENT_TRAY_UPLOAD, () => cb());
}

export function platform(): AppInfo["platform"] {
  const p = import.meta.env.TAURI_ENV_PLATFORM as string | undefined;
  if (p === "macos" || p === "windows" || p === "linux" || p === "android" || p === "ios") return p;
  return "web";
}
