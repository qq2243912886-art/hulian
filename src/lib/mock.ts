/**
 * 互联 · Mock 实现：`VITE_MOCK=1 pnpm dev` 时替换所有 Tauri 命令，
 * 让 UI 可以在浏览器里独立开发/预览（不依赖 Rust、不碰真实云盘）。
 */
import type { AppConfig, DriveQuota, RemoteFile, TransferItem } from "./types";

const now = Date.now();
const MB = 1024 * 1024;

let config: AppConfig = {
  account: {
    endpoint: "bjxxxxx.api.aliyunfile.com",
    domainId: "bjxxxxx",
    driveId: "2",
    accessToken: "uk-mock-token",
    userName: "演示用户",
    driveName: "示例空间",
  },
  settings: {
    transferFolderName: "文件互传",
    downloadDir: "/Users/mock/Downloads/互联",
    chunkSizeMb: 8,
    parallelParts: 1,
    parallelTransfers: 2,
    autostart: false,
    closeToTray: true,
    deviceName: "MacBook · 演示用户",
    theme: "dark",
  },
};

let remote: RemoteFile[] = [
  {
    fileId: "mock-1",
    name: "产品原型-v3.fig",
    size: 24.6 * MB,
    isFolder: false,
    createdAt: new Date(now - 6 * 60_000).toISOString(),
    updatedAt: new Date(now - 6 * 60_000).toISOString(),
    parentFileId: "root",
  },
  {
    fileId: "mock-2",
    name: "会议录音-20261020.m4a",
    size: 88.2 * MB,
    isFolder: false,
    createdAt: new Date(now - 52 * 60_000).toISOString(),
    updatedAt: new Date(now - 52 * 60_000).toISOString(),
    parentFileId: "root",
  },
  {
    fileId: "mock-3",
    name: "iPhone-截图-1019.png",
    size: 3.4 * MB,
    isFolder: false,
    createdAt: new Date(now - 26 * 3600_000).toISOString(),
    updatedAt: new Date(now - 26 * 3600_000).toISOString(),
    parentFileId: "root",
  },
];

let transfers: TransferItem[] = [
  {
    id: "t-demo-1",
    kind: "download",
    name: "产品原型-v3.fig",
    localPath: "/Users/mock/Downloads/互联/产品原型-v3.fig",
    size: 24.6 * MB,
    transferred: 24.6 * MB,
    state: "success",
    speedBps: 0,
    remoteFileId: "mock-1",
    createdAt: now - 3600_000,
    finishedAt: now - 3550_000,
  },
];

const updateListeners = new Set<(item: TransferItem) => void>();
const listListeners = new Set<(items: TransferItem[]) => void>();
const timers = new Map<string, ReturnType<typeof setInterval>>();

function emit(item: TransferItem) {
  updateListeners.forEach((cb) => cb(item));
}

function startFake(id: string) {
  const item = transfers.find((t) => t.id === id);
  if (!item) return;
  item.state = "running";
  let done = 0;
  const total = item.size || 4 * MB;
  const speed = 6.5 * MB;
  const tick = setInterval(() => {
    done += speed * 0.25;
    item.transferred = Math.min(done, total);
    item.speedBps = speed;
    if (item.transferred >= total) {
      item.state = "success";
      item.speedBps = 0;
      item.finishedAt = Date.now();
      clearInterval(tick);
      timers.delete(id);
    }
    emit({ ...item });
  }, 250);
  timers.set(id, tick);
}

function addTask(kind: "upload" | "download", name: string, size: number, localPath: string): string {
  const id = `t-${Math.random().toString(36).slice(2, 8)}`;
  const item: TransferItem = {
    id,
    kind,
    name,
    localPath,
    size,
    transferred: 0,
    state: "queued",
    speedBps: 0,
    createdAt: Date.now(),
  };
  transfers = [item, ...transfers];
  listListeners.forEach((cb) => cb([...transfers]));
  setTimeout(() => startFake(id), 300);
  return id;
}

const devices = ["MacBook · 演示用户", "Windows · 办公室", "Pixel · 随身"];

const driveQuota: DriveQuota = { driveName: "示例空间", usedSize: 2.67 * 1024 * MB, totalSize: 400 * 1024 * MB };

export async function invokeMock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const wait = (ms = 260) => new Promise((r) => setTimeout(r, ms));
  switch (cmd) {
    case "get_config":
      await wait(80);
      return config as unknown as T;
    case "save_config":
      config = args?.config as AppConfig;
      await wait(120);
      return config as unknown as T;
    case "import_local_cli_config":
      await wait(500);
      return { ok: true, account: config.account } as unknown as T;
    case "test_connection":
      await wait(700);
      return {
        ok: true,
        message: "连接正常",
        user: "演示用户",
        drive: "示例空间",
        quota: driveQuota,
      } as unknown as T;
    case "get_drive_quota":
      await wait(200);
      return driveQuota as unknown as T;
    case "list_remote":
    case "list_remote_folders":
      await wait(320);
      return remote as unknown as T;
    case "list_folder":
      await wait(240);
      return remote as unknown as T;
    case "upload_files": {
      const paths = (args?.paths as string[]) ?? [];
      await wait(200);
      return paths.map((p) => {
        const name = p.split("/").pop() ?? p;
        return addTask("upload", name, (2 + Math.random() * 30) * MB, p);
      }) as unknown as T;
    }
    case "pick_and_upload":
      await wait(200);
      return [addTask("upload", "拖进来的文件.zip", 12.4 * MB, "/Users/mock/Desktop/拖进来的文件.zip")] as unknown as T;
    case "download_files": {
      const files = (args?.files as { fileId: string; name: string }[]) ?? [];
      return files.map((f) => {
        const rf = remote.find((r) => r.fileId === f.fileId);
        return addTask("download", f.name, rf?.size ?? 4 * MB, `${config.settings.downloadDir}/${f.name}`);
      }) as unknown as T;
    }
    case "list_transfers":
      return [...transfers] as unknown as T;
    case "cancel_transfer": {
      const t = transfers.find((x) => x.id === args?.id);
      if (t) {
        t.state = "canceled";
        const timer = timers.get(t.id);
        if (timer) clearInterval(timer);
        timers.delete(t.id);
        emit({ ...t });
      }
      return undefined as unknown as T;
    }
    case "retry_transfer": {
      const t = transfers.find((x) => x.id === args?.id);
      if (t) {
        t.state = "queued";
        t.transferred = 0;
        emit({ ...t });
        setTimeout(() => startFake(t.id), 300);
      }
      return ((args?.id as string) ?? "") as unknown as T;
    }
    case "remove_transfer":
      transfers = transfers.filter((x) => x.id !== args?.id);
      listListeners.forEach((cb) => cb([...transfers]));
      return undefined as unknown as T;
    case "clear_finished_transfers":
      transfers = transfers.filter((x) => x.state === "running" || x.state === "queued");
      listListeners.forEach((cb) => cb([...transfers]));
      return undefined as unknown as T;
    case "open_path":
    case "reveal_in_folder":
    case "open_download_dir":
    case "hide_main_window":
    case "show_main_window":
    case "quit_app":
    case "set_autostart":
      await wait(60);
      return undefined as unknown as T;
    case "get_app_info":
      await wait(60);
      return {
        version: "0.1.0",
        platform: "web",
        deviceName: config.settings.deviceName,
        downloadDir: config.settings.downloadDir,
        isDesktop: false,
      } as unknown as T;
    default:
      throw new Error(`mock 未实现的命令: ${cmd}`);
  }
}

export async function onMockTransferUpdate(cb: (item: TransferItem) => void): Promise<() => void> {
  updateListeners.add(cb);
  return () => updateListeners.delete(cb);
}

export async function onMockTransferList(cb: (items: TransferItem[]) => void): Promise<() => void> {
  listListeners.add(cb);
  return () => listListeners.delete(cb);
}

export { devices };
