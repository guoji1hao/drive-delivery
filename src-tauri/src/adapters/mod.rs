pub mod baidu;
pub mod quark;

use crate::{
    errors::AppResult,
    models::{AuthStatus, ShareResult},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgress {
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub upload_id: Option<String>,
    pub resume_data: Option<String>,
}

pub type ProgressSink = Arc<dyn Fn(UploadProgress) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct UploadRequest {
    pub task_id: String,
    pub local_path: PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub remote_dir: String,
    pub resume_data: Option<String>,
    pub retry_limit: u32,
}

#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub remote_path: String,
    pub remote_id: Option<String>,
    pub upload_id: Option<String>,
    pub resume_data: Option<String>,
    pub instant_upload: bool,
}

#[async_trait]
#[allow(dead_code)]
pub trait DriveAdapter: Send + Sync {
    fn provider(&self) -> &'static str;
    async fn authenticate(&self) -> AppResult<AuthStatus>;
    async fn refresh_token(&self) -> AppResult<()>;
    async fn upload_file(
        &self,
        request: UploadRequest,
        progress: ProgressSink,
        cancel: CancellationToken,
    ) -> AppResult<UploadOutcome>;
    async fn upload_folder(&self, _path: PathBuf) -> AppResult<Vec<UploadOutcome>>;
    async fn get_upload_progress(&self, _task_id: &str) -> AppResult<Option<UploadProgress>>;
    async fn create_share(
        &self,
        outcome: &UploadOutcome,
        period_days: u32,
    ) -> AppResult<ShareResult>;
    async fn get_share_result(&self, share: &ShareResult) -> AppResult<ShareResult>;
    async fn cancel_upload(&self, task_id: &str) -> AppResult<()>;
    async fn retry_upload(
        &self,
        request: UploadRequest,
        progress: ProgressSink,
        cancel: CancellationToken,
    ) -> AppResult<UploadOutcome> {
        self.upload_file(request, progress, cancel).await
    }
}
