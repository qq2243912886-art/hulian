/**
 * 互联 · 前后端契约类型（Frontend ⇄ Rust）
 * 这份文件是唯一的接口定义来源：任何字段变更都需要同步 src-tauri/src/model.rs。
 */

/** 云盘账号凭据（阿里云盘个人版 / PDS） */
export interface PdsAccount {
  /** 形如 bjxxxxx.api.aliyunfile.com */
  endpoint: string;
  /** 形如 bjxxxxx */
  domainId: string;
  /** 形如 2 */
  driveId: string;
  /** 形如 uk-xxxxxxxx（个人访问令牌） */
  accessToken: string;
  /** 以下为连接后回填的展示信息 */
  userId?: string;
  userName?: string;
  driveName?: string;
}

export type ThemeMode = "dark" | "light" | "system";

/** 用户可调设置 */
export interface AppSettings {
  /** 云端互传目录名（根目录下自动创建），默认「文件互传」 */
  transferFolderName: string;
  /** 本地下载目录（为空时由 Rust 端给出系统默认值） */
  downloadDir: string;
  /** 上传分片大小（MB），桌面默认 8，移动端默认 4 */
  chunkSizeMb: number;
  /** 单个文件内并发分片数 */
  parallelParts: number;
  /** 同时进行的传输任务数 */
  parallelTransfers: number;
  /** 开机自启（桌面端） */
  autostart: boolean;
  /** 关闭窗口时驻留托盘（桌面端）；false = 直接退出 */
  closeToTray: boolean;
  /** 本机设备名，用于传输记录标注来源 */
  deviceName: string;
  theme: ThemeMode;
}

export interface AppConfig {
  account: PdsAccount | null;
  settings: AppSettings;
}

export interface DriveQuota {
  driveName: string;
  usedSize: number;
  totalSize: number;
}

export interface RemoteFile {
  fileId: string;
  name: string;
  size: number;
  isFolder: boolean;
  /** ISO8601 字符串 */
  createdAt: string;
  updatedAt: string;
  parentFileId: string;
}

export type TransferKind = "upload" | "download";
export type TransferState = "queued" | "running" | "success" | "error" | "canceled";

export interface TransferItem {
  id: string;
  kind: TransferKind;
  /** 文件名（云端名） */
  name: string;
  /** 本地绝对路径（上传源 / 下载目标） */
  localPath: string;
  size: number;
  /** 已传输字节 */
  transferred: number;
  state: TransferState;
  /** 瞬时速度 B/s（running 时有效） */
  speedBps: number;
  error?: string;
  remoteFileId?: string;
  /** unix 毫秒 */
  createdAt: number;
  finishedAt?: number;
}

export interface ConnectionStatus {
  ok: boolean;
  message?: string;
  user?: string;
  drive?: string;
  quota?: DriveQuota;
}

/** 从本机 aliyun CLI 配置导入的结果（仅桌面端） */
export interface ImportResult {
  ok: boolean;
  account?: PdsAccount;
  message?: string;
}

export interface AppInfo {
  version: string;
  platform: "macos" | "windows" | "linux" | "android" | "ios" | "web";
  deviceName: string;
  downloadDir: string;
  /** 桌面端为 true（支持托盘、自启、拖拽文件路径） */
  isDesktop: boolean;
}

/** 事件名 */
export const EVENT_TRANSFER_UPDATE = "transfer://update";
export const EVENT_TRANSFER_LIST = "transfer://list";
export const EVENT_TRAY_UPLOAD = "app://tray-upload";
export const EVENT_REQUEST_QUIT = "app://request-quit";

export type TransferListener = (item: TransferItem) => void;
export type VoidListener = () => void;
