/** 互联 · 展示层小工具（无副作用、无状态） */
import type { TransferItem, TransferState } from "./types";

export interface TransferStateMeta {
  label: string;
  tone: "muted" | "info" | "success" | "danger";
}

const STATE_META: Record<TransferState, TransferStateMeta> = {
  queued: { label: "等待中", tone: "muted" },
  running: { label: "传输中", tone: "info" },
  success: { label: "已完成", tone: "success" },
  error: { label: "失败", tone: "danger" },
  canceled: { label: "已取消", tone: "muted" },
};

export function transferStateMeta(item: TransferItem): TransferStateMeta {
  if (item.state === "success") {
    return {
      label: item.kind === "upload" ? "已上传" : "已下载",
      tone: "success",
    };
  }
  return STATE_META[item.state];
}

/** 0–1 的进度（size 为 0 时按状态兜底） */
export function progressOf(item: TransferItem): number {
  if (!Number.isFinite(item.size) || item.size <= 0) {
    return item.state === "success" ? 1 : 0;
  }
  return Math.min(1, Math.max(0, item.transferred / item.size));
}

/** 展示用百分比：运行中向下取整，避免 99.6% 显示成 100% */
export function percentOf(item: TransferItem): number {
  if (item.state === "success") return 100;
  return Math.floor(progressOf(item) * 100);
}

/** 文件名截断（toast / 空间紧张处） */
export function shortName(name: string, max = 22): string {
  if (name.length <= max) return name;
  const dot = name.lastIndexOf(".");
  if (dot > 0 && name.length - dot <= 8) {
    const ext = name.slice(dot);
    return `${name.slice(0, max - ext.length - 1)}…${ext}`;
  }
  return `${name.slice(0, max - 1)}…`;
}

/** 传输方向文案 */
export function kindLabel(item: Pick<TransferItem, "kind">): string {
  return item.kind === "upload" ? "上传" : "下载";
}
