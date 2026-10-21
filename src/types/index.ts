export type PlatformState =
  | "waiting"
  | "hashing"
  | "uploading"
  | "creating_share"
  | "completed"
  | "paused"
  | "cancelled"
  | "failed"
  | "retryable"
  | "skipped"
  | "unavailable";

export type TaskState =
  | "waiting"
  | "processing"
  | "partial_success"
  | "completed"
  | "paused"
  | "cancelled"
  | "failed"
  | "duplicate";

export type Provider = "baidu" | "google" | "onedrive";

export interface ShareResult {
  url?: string;
  code?: string;
  expiresAt?: number;
  shareId?: string;
  remotePath?: string;
  notice?: string;
}

export interface PlatformTask {
  state: PlatformState;
  progress: number;
  uploadedBytes: number;
  totalBytes: number;
  remotePath?: string;
  remoteId?: string;
  uploadId?: string;
  resumeData?: string;
  share: ShareResult;
  error?: string;
  retryCount: number;
}

export interface UploadTask {
  id: string;
  batchId: string;
  localPath: string;
  fileName: string;
  fileSize: number;
  fileHash: string;
  createdAt: number;
  updatedAt: number;
  state: TaskState;
  message?: string;
  baidu: PlatformTask;
  google: PlatformTask;
  onedrive: PlatformTask;
}

export interface HistoryRecord {
  id: string;
  fileName: string;
  fileSize: number;
  localPath: string;
  fileHash: string;
  uploadedAt: number;
  taskStatus: TaskState;
  baiduPath?: string;
  baiduRemoteId?: string;
  baiduUrl?: string;
  baiduCode?: string;
  baiduExpiresAt?: number;
  baiduShareId?: string;
  googlePath?: string;
  onedrivePath?: string;
  googleRemoteId?: string;
  onedriveRemoteId?: string;
  googleUrl?: string;
  onedriveUrl?: string;
  googleCode?: string;
  onedriveCode?: string;
  googleExpiresAt?: number;
  onedriveExpiresAt?: number;
  googleShareId?: string;
  onedriveShareId?: string;
}

export interface AppSettings {
  baiduDefaultDir: string;
  googleDefaultDir: string;
  onedriveDefaultDir: string;
  datedSubdirectory: boolean;
  maxConcurrentFiles: number;
  retryLimit: number;
  sharePeriodDays: number;
  copyTemplate: string;
  autoCopy: boolean;
}

export interface AuthStatus {
  provider: Provider;
  authenticated: boolean;
  available: boolean;
  expiresAt?: number;
  message: string;
}

export interface AuthChallenge {
  provider: string;
  verificationUrl: string;
  userCode?: string;
  deviceCode: string;
  expiresIn: number;
  interval: number;
}

export interface AuthPollResult {
  pending: boolean;
  authenticated: boolean;
  message: string;
  expiresAt?: number;
}

export interface CapabilityReport {
  provider: Provider;
  oauth: string;
  upload: string;
  resume: string;
  share: string;
  limitations: string[];
}

export interface DrivePlugin {
  manifest: { id: Provider; apiVersion: number; version: string };
  installed: boolean;
  enabled: boolean;
}

export interface AppSnapshot {
  plugins: DrivePlugin[];
  tasks: UploadTask[];
  settings: AppSettings;
  auth: AuthStatus[];
  capabilities: CapabilityReport[];
  dataDir: string;
  logDir: string;
}

export interface EnqueueResult {
  accepted: number;
  duplicates: number;
  tasks: UploadTask[];
}
