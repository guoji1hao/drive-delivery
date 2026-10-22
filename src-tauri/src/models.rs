use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformState {
    Waiting,
    Hashing,
    Uploading,
    CreatingShare,
    Completed,
    Paused,
    Cancelled,
    Failed,
    Retryable,
    Unavailable,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Waiting,
    Processing,
    PartialSuccess,
    Completed,
    Paused,
    Cancelled,
    Failed,
    Duplicate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShareResult {
    pub url: Option<String>,
    pub code: Option<String>,
    pub expires_at: Option<i64>,
    pub share_id: Option<String>,
    pub remote_path: Option<String>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformTask {
    pub state: PlatformState,
    pub progress: f64,
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub remote_path: Option<String>,
    pub remote_id: Option<String>,
    pub upload_id: Option<String>,
    pub resume_data: Option<String>,
    pub share: ShareResult,
    pub error: Option<String>,
    pub retry_count: u32,
}

impl PlatformTask {
    pub fn waiting(total_bytes: u64) -> Self {
        Self {
            state: PlatformState::Waiting,
            progress: 0.0,
            uploaded_bytes: 0,
            total_bytes,
            remote_path: None,
            remote_id: None,
            upload_id: None,
            resume_data: None,
            share: ShareResult::default(),
            error: None,
            retry_count: 0,
        }
    }
}

impl Default for PlatformTask {
    fn default() -> Self {
        Self::waiting(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadTask {
    #[serde(default)]
    pub destination_dirs: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub relative_dir: String,
    #[serde(default)]
    pub is_directory: bool,
    pub id: String,
    pub batch_id: String,
    pub local_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub state: TaskState,
    pub message: Option<String>,
    pub baidu: PlatformTask,
    #[serde(default)]
    pub google: PlatformTask,
    #[serde(default)]
    pub onedrive: PlatformTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub local_path: String,
    pub file_hash: String,
    pub uploaded_at: i64,
    pub task_status: TaskState,
    pub baidu_path: Option<String>,
    pub baidu_remote_id: Option<String>,
    pub baidu_url: Option<String>,
    pub baidu_code: Option<String>,
    pub baidu_expires_at: Option<i64>,
    pub baidu_share_id: Option<String>,
    pub google_path: Option<String>,
    pub onedrive_path: Option<String>,
    pub google_remote_id: Option<String>,
    pub onedrive_remote_id: Option<String>,
    pub google_url: Option<String>,
    pub onedrive_url: Option<String>,
    pub google_code: Option<String>,
    pub onedrive_code: Option<String>,
    pub google_expires_at: Option<i64>,
    pub onedrive_expires_at: Option<i64>,
    pub google_share_id: Option<String>,
    pub onedrive_share_id: Option<String>,
}

impl From<&UploadTask> for HistoryRecord {
    fn from(task: &UploadTask) -> Self {
        Self {
            id: task.id.clone(),
            file_name: task.file_name.clone(),
            file_size: task.file_size,
            local_path: task.local_path.clone(),
            file_hash: task.file_hash.clone(),
            uploaded_at: task.updated_at,
            task_status: task.state.clone(),
            baidu_path: task.baidu.remote_path.clone(),
            baidu_remote_id: task.baidu.remote_id.clone(),
            baidu_url: task.baidu.share.url.clone(),
            baidu_code: task.baidu.share.code.clone(),
            baidu_expires_at: task.baidu.share.expires_at,
            baidu_share_id: task.baidu.share.share_id.clone(),
            google_path: task.google.remote_path.clone(),
            onedrive_path: task.onedrive.remote_path.clone(),
            google_remote_id: task.google.remote_id.clone(),
            onedrive_remote_id: task.onedrive.remote_id.clone(),
            google_url: task.google.share.url.clone(),
            onedrive_url: task.onedrive.share.url.clone(),
            google_code: task.google.share.code.clone(),
            onedrive_code: task.onedrive.share.code.clone(),
            google_expires_at: task.google.share.expires_at,
            onedrive_expires_at: task.onedrive.share.expires_at,
            google_share_id: task.google.share.share_id.clone(),
            onedrive_share_id: task.onedrive.share.share_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub baidu_default_dir: String,
    pub google_default_dir: String,
    pub onedrive_default_dir: String,
    pub dated_subdirectory: bool,
    pub max_concurrent_files: u32,
    pub retry_limit: u32,
    pub share_period_days: u32,
    pub copy_template: String,
    pub auto_copy: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            baidu_default_dir: "/apps/双盘发货助手/自动发货/".to_string(),
            google_default_dir: "/自动发货/".to_string(),
            onedrive_default_dir: "/自动发货/".to_string(),
            dated_subdirectory: false,
            max_concurrent_files: 2,
            retry_limit: 3,
            share_period_days: 0,
            copy_template: concat!(
                "文件：{filename}\n",
                "百度网盘：\n{baidu_url}\n{baidu_code_line}\n",
                "Google Drive：\n{google_url}\n{google_code_line}\n",
                "OneDrive：\n{onedrive_url}\n{onedrive_code_line}"
            )
            .to_string(),
            auto_copy: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub provider: String,
    pub authenticated: bool,
    pub available: bool,
    pub expires_at: Option<i64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallenge {
    pub provider: String,
    pub verification_url: String,
    pub user_code: Option<String>,
    pub device_code: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthPollResult {
    pub pending: bool,
    pub authenticated: bool,
    pub message: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityReport {
    pub provider: String,
    pub oauth: String,
    pub upload: String,
    pub resume: String,
    pub share: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub queue: crate::queue::QueueStatus,
    pub plugins: Vec<crate::plugins::DrivePlugin>,
    pub tasks: Vec<UploadTask>,
    pub settings: AppSettings,
    pub auth: Vec<AuthStatus>,
    pub capabilities: Vec<CapabilityReport>,
    pub data_dir: String,
    pub log_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialsInput {
    pub app_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCredentialsInput {
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueResult {
    pub accepted: usize,
    pub duplicates: usize,
    pub tasks: Vec<UploadTask>,
}
