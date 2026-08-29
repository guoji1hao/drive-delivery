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
  quark: PlatformTask;
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
  quarkPath?: string;
  quarkRemoteId?: string;
  quarkUrl?: string;
  quarkCode?: string;
  quarkExpiresAt?: number;
  quarkShareId?: string;
}

export interface AppSettings {
  baiduDefaultDir: string;
  quarkDefaultDir: string;
  quarkCliScript: string;
  datedSubdirectory: boolean;
  maxConcurrentFiles: number;
  retryLimit: number;
  sharePeriodDays: number;
  copyTemplate: string;
  autoCopy: boolean;
}

export interface AuthStatus {
  provider: "baidu" | "quark";
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
  provider: "baidu" | "quark";
  oauth: string;
  upload: string;
  resume: string;
  share: string;
  limitations: string[];
}

export interface AppSnapshot {
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
