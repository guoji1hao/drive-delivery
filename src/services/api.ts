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
  startQuarkAuth: () => invoke<string>("start_quark_auth"),
  pauseTask: (taskId: string, provider?: string) =>
    invoke<UploadTask>("pause_task", { taskId, provider }),
  cancelTask: (taskId: string, provider?: string) =>
    invoke<UploadTask>("cancel_task", { taskId, provider }),
  retryTask: (taskId: string, provider?: string) =>
    invoke<UploadTask>("retry_task", { taskId, provider }),
  searchHistory: (query: string) =>
    invoke<HistoryRecord[]>("search_history", { query }),
  copyTaskResult: (taskId: string, provider?: string) =>
    invoke<string>("copy_task_result", { taskId, provider }),
  copyHistoryResult: (record: HistoryRecord) =>
    invoke<string>("copy_history_result", { record }),
};
