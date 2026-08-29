import type { PlatformState, TaskState } from "../types";

export const platformStateText: Record<PlatformState, string> = {
  waiting: "等待上传",
  hashing: "校验文件",
  uploading: "上传中",
  creating_share: "创建分享中",
  completed: "已完成",
  paused: "已暂停",
  cancelled: "已取消",
  failed: "失败",
  retryable: "失败，可重试",
  unavailable: "官方接入不可用",
};

export const taskStateText: Record<TaskState, string> = {
  waiting: "等待处理",
  processing: "处理中",
  partial_success: "部分完成",
  completed: "已完成",
  paused: "已暂停",
  cancelled: "已取消",
  failed: "处理失败",
  duplicate: "历史记录",
};

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** index;
  return `${value >= 100 || index === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
}

export function formatDate(timestamp?: number): string {
  if (!timestamp) return "永久有效";
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1000));
}

export function isBusy(state: PlatformState): boolean {
  return ["hashing", "uploading", "creating_share"].includes(state);
}

export function canRetry(state: PlatformState): boolean {
  return ["paused", "cancelled", "failed", "retryable", "unavailable"].includes(
    state,
  );
}
