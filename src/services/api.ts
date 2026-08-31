import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AppSnapshot,
  AuthChallenge,
  AuthPollResult,
  AuthStatus,
  EnqueueResult,
  HistoryRecord,
  Provider,
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
  saveGoogleCredentials: (clientId: string, clientSecret?: string) =>
    invoke<AuthStatus>("save_google_credentials", {
      credentials: { clientId, clientSecret },
    }),
  startGoogleAuth: () => invoke<AuthChallenge>("start_google_auth"),
  pollGoogleAuth: (deviceCode: string) =>
    invoke<AuthPollResult>("poll_google_auth", { deviceCode }),
  clearGoogleAuth: () => invoke<AuthStatus>("clear_google_auth"),
  pauseTask: (taskId: string, provider: Provider) =>
    invoke<UploadTask>("pause_task", { taskId, provider }),
  cancelTask: (taskId: string, provider: Provider) =>
    invoke<UploadTask>("cancel_task", { taskId, provider }),
  retryTask: (taskId: string, provider: Provider) =>
    invoke<UploadTask>("retry_task", { taskId, provider }),
  searchHistory: (query: string) =>
    invoke<HistoryRecord[]>("search_history", { query }),
  copyTaskResult: (taskId: string, provider: Provider | "all" = "all") =>
    invoke<string>("copy_task_result", { taskId, provider }),
  copyHistoryResult: (
    record: HistoryRecord,
    provider: Provider | "all" = "all",
  ) => invoke<string>("copy_history_result", { record, provider }),
};
