import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AppSnapshot,
  AuthChallenge,
  AuthPollResult,
  AuthStatus,
  EnqueueResult,
  HistoryRecord,
  UploadTask,
} from "../types";

export const api = {
  initialize: () => invoke<AppSnapshot>("initialize_app"),
  enqueue: (paths: string[]) =>
    invoke<EnqueueResult>("enqueue_paths", { paths }),
  saveSettings: (settings: AppSettings) =>
    invoke<AppSettings>("save_settings", { settings }),
  saveBaiduCredentials: (appKey: string, secretKey: string) =>
    invoke<AuthStatus>("save_baidu_credentials", {
      credentials: { appKey, secretKey },
    }),
  startBaiduAuth: () => invoke<AuthChallenge>("start_baidu_auth"),
  pollBaiduAuth: (deviceCode: string) =>
    invoke<AuthPollResult>("poll_baidu_auth", { deviceCode }),
  clearBaiduAuth: () => invoke<AuthStatus>("clear_baidu_auth"),
  pauseTask: (taskId: string) =>
    invoke<UploadTask>("pause_task", { taskId }),
  cancelTask: (taskId: string) =>
    invoke<UploadTask>("cancel_task", { taskId }),
  retryTask: (taskId: string) =>
    invoke<UploadTask>("retry_task", { taskId }),
  searchHistory: (query: string) =>
    invoke<HistoryRecord[]>("search_history", { query }),
  copyTaskResult: (taskId: string) =>
    invoke<string>("copy_task_result", { taskId }),
  copyHistoryResult: (record: HistoryRecord) =>
    invoke<string>("copy_history_result", { record }),
};
