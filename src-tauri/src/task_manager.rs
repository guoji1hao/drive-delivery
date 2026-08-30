use crate::{
    adapters::{
        DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest,
        baidu::BaiduDriveAdapter,
    },
    clipboard,
    database::Database,
    errors::{AppError, AppResult},
    logging::LogService,
    models::{
        AppSettings, EnqueueResult, HistoryRecord, PlatformState, PlatformTask, ShareResult,
        TaskState, UploadTask,
    },
    settings::SettingsService,
    share_manager,
};
use chrono::{Local, Utc};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{AppHandle, Emitter};
use tokio::{
    sync::{Mutex as AsyncMutex, Semaphore},
    time::{Duration, sleep},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use walkdir::WalkDir;

pub struct TaskManager {
    app: AppHandle,
    database: Arc<Database>,
    settings: Arc<SettingsService>,
    baidu: Arc<BaiduDriveAdapter>,
    log: Arc<LogService>,
    cancellation: AsyncMutex<HashMap<String, CancellationToken>>,
}

impl TaskManager {
    pub fn new(
        app: AppHandle,
        database: Arc<Database>,
        settings: Arc<SettingsService>,
        baidu: Arc<BaiduDriveAdapter>,
        log: Arc<LogService>,
    ) -> Self {
        Self {
            app,
            database,
            settings,
            baidu,
            log,
            cancellation: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn enqueue_paths(self: &Arc<Self>, paths: Vec<String>) -> AppResult<EnqueueResult> {
        let path_bufs = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
        let files = tokio::task::spawn_blocking(move || expand_files(path_bufs))
            .await
            .map_err(|error| AppError::File(format!("展开文件夹失败：{error}")))??;
        if files.is_empty() {
            return Err(AppError::File("拖入内容中没有可读取的文件".to_string()));
        }
        let batch_id = Uuid::new_v4().to_string();
        let settings = self.settings.get()?;
        let semaphore = Arc::new(Semaphore::new(settings.max_concurrent_files as usize));
        let mut tasks = Vec::new();
        let mut duplicates = 0;
        for path in files {
            let metadata = tokio::fs::metadata(&path)
                .await
                .map_err(|error| AppError::File(format!("无法读取 {}：{error}", path.display())))?;
            let size = metadata.len();
            let hash_path = path.clone();
            let hash = tokio::task::spawn_blocking(move || sha256_file(&hash_path))
                .await
                .map_err(|error| AppError::File(format!("计算文件 Hash 失败：{error}")))??;
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| AppError::File("文件名不是有效 UTF-8".to_string()))?
                .to_string();
            let now = Utc::now().timestamp();
            let history = self.database.find_history_by_hash(&hash, size)?;
            let task = if let Some(history) = history {
                duplicates += 1;
                duplicate_task(&batch_id, &path, file_name, size, hash, now, history)
            } else {
                UploadTask {
                    id: Uuid::new_v4().to_string(),
                    batch_id: batch_id.clone(),
                    local_path: path.to_string_lossy().to_string(),
                    file_name,
                    file_size: size,
                    file_hash: hash,
                    created_at: now,
                    updated_at: now,
                    state: TaskState::Waiting,
                    message: None,
                    baidu: PlatformTask::waiting(size),
                }
            };
            let should_start = task.state != TaskState::Duplicate;
            self.database.save_task(&task)?;
            self.emit_task(&task);
            tasks.push(task.clone());
            if should_start {
                let manager = Arc::clone(self);
                let semaphore = Arc::clone(&semaphore);
                tokio::spawn(async move {
                    let permit = semaphore.acquire_owned().await;
                    if permit.is_ok() {
                        let _ = manager.process_task(&task.id).await;
                    }
                });
            }
        }
        Ok(EnqueueResult {
            accepted: tasks.len(),
            duplicates,
            tasks,
        })
    }

    pub async fn pause(&self, task_id: &str) -> AppResult<UploadTask> {
        self.set_terminal_control_state(task_id, PlatformState::Paused, TaskState::Paused)
            .await
    }

    pub async fn cancel(&self, task_id: &str) -> AppResult<UploadTask> {
        self.set_terminal_control_state(task_id, PlatformState::Cancelled, TaskState::Cancelled)
            .await
    }

    pub async fn retry(self: &Arc<Self>, task_id: &str) -> AppResult<UploadTask> {
        let mut task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        if task.baidu.state != PlatformState::Completed {
            task.baidu.state = PlatformState::Waiting;
            task.baidu.error = None;
        }
        task.state = TaskState::Waiting;
        task.updated_at = Utc::now().timestamp();
        self.database.save_task(&task)?;
        self.emit_task(&task);
        let manager = Arc::clone(self);
        let task_id = task_id.to_string();
        tokio::spawn(async move {
            let _ = manager.process_task(&task_id).await;
        });
        Ok(task)
    }

    pub fn list_tasks(&self) -> AppResult<Vec<UploadTask>> {
        self.database.list_tasks(500)
    }

    pub fn search_history(&self, query: &str) -> AppResult<Vec<HistoryRecord>> {
        self.database.search_history(query, 500)
    }

    async fn set_terminal_control_state(
        &self,
        task_id: &str,
        platform_state: PlatformState,
        task_state: TaskState,
    ) -> AppResult<UploadTask> {
        let mut task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        if task.baidu.state != PlatformState::Completed {
            task.baidu.state = platform_state;
        }
        task.state = task_state;
        task.updated_at = Utc::now().timestamp();
        self.database.save_task(&task)?;
        self.emit_task(&task);
        let tokens = self.cancellation.lock().await;
        if let Some(token) = tokens.get(task_id) {
            token.cancel();
        }
        Ok(task)
    }

    async fn process_task(&self, task_id: &str) -> AppResult<()> {
        let task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        let shared = Arc::new(Mutex::new(task));
        {
            let mut task = shared.lock();
            task.state = TaskState::Processing;
            task.message = None;
            task.updated_at = Utc::now().timestamp();
        }
        self.publish(&shared)?;
        let settings = self.settings.get()?;
        let _ = self
            .process_baidu_with_retries(Arc::clone(&shared), &settings)
            .await;
        {
            let mut task = shared.lock();
            task.state = overall_state(&task);
            task.updated_at = Utc::now().timestamp();
            if task.state == TaskState::Completed {
                task.message = Some("百度网盘处理完成".to_string());
            }
        }
        let final_task = self.publish(&shared)?;
        if matches!(
            final_task.state,
            TaskState::Completed | TaskState::Duplicate
        ) {
            self.database.save_history(&final_task)?;
            if settings.auto_copy {
                let text = share_manager::format_task(&final_task, &settings);
                if text.contains("http") {
                    let _ = clipboard::copy_text(&text);
                }
            }
        }
        Ok(())
    }

    async fn process_baidu_with_retries(
        &self,
        task: Arc<Mutex<UploadTask>>,
        settings: &AppSettings,
    ) -> AppResult<()> {
        let mut retry = 0_u32;
        loop {
            match self.process_baidu(Arc::clone(&task), settings).await {
                Ok(()) => return Ok(()),
                Err(error)
                    if matches!(error, AppError::Provider(_)) && retry < settings.retry_limit =>
                {
                    retry += 1;
                    let delay = 1_u64 << (retry - 1).min(5);
                    self.log.info(
                        "baidu",
                        &format!("平台任务失败，{delay} 秒后自动执行第 {retry} 次重试"),
                    );
                    sleep(Duration::from_secs(delay)).await;

                    let task_id = task.lock().id.clone();
                    if let Some(persisted) = self.database.get_task(&task_id)? {
                        let requested = persisted.baidu.state.clone();
                        if matches!(requested, PlatformState::Paused | PlatformState::Cancelled) {
                            let mut task = task.lock();
                            task.baidu.state = requested;
                            return Err(AppError::Paused);
                        }
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn process_baidu(
        &self,
        task: Arc<Mutex<UploadTask>>,
        settings: &AppSettings,
    ) -> AppResult<()> {
        let (task_id, local_path, file_name, file_size, existing, previous_state) = {
            let task = task.lock();
            (
                task.id.clone(),
                PathBuf::from(&task.local_path),
                task.file_name.clone(),
                task.file_size,
                task.baidu.clone(),
                task.baidu.state.clone(),
            )
        };
        if previous_state == PlatformState::Completed {
            return Ok(());
        }
        let adapter: Arc<dyn DriveAdapter> = self.baidu.clone();
        let cancel = CancellationToken::new();
        let cancel_key = task_id.clone();
        self.cancellation
            .lock()
            .await
            .insert(cancel_key.clone(), cancel.clone());
        let outcome_from_history = existing
            .remote_path
            .clone()
            .map(|remote_path| UploadOutcome {
                remote_path,
                remote_id: existing.remote_id.clone(),
                upload_id: existing.upload_id.clone(),
                resume_data: existing.resume_data.clone(),
                instant_upload: true,
            });
        let should_share_only = outcome_from_history.is_some()
            && matches!(
                previous_state,
                PlatformState::CreatingShare | PlatformState::Retryable
            );
        let remote_dir = dated_dir(&settings.baidu_default_dir, settings.dated_subdirectory);
        let progress_task = Arc::clone(&task);
        let database = Arc::clone(&self.database);
        let app = self.app.clone();
        let progress: ProgressSink = Arc::new(move |value: UploadProgress| {
            let snapshot = {
                let mut task = progress_task.lock();
                let platform = &mut task.baidu;
                platform.state = PlatformState::Uploading;
                platform.uploaded_bytes = value.uploaded_bytes;
                if value.total_bytes > 0 {
                    platform.total_bytes = value.total_bytes;
                }
                platform.progress = value.percent.clamp(0.0, 100.0);
                if value.upload_id.is_some() {
                    platform.upload_id = value.upload_id;
                }
                if value.resume_data.is_some() {
                    platform.resume_data = value.resume_data;
                }
                task.updated_at = Utc::now().timestamp();
                task.clone()
            };
            let _ = database.save_task(&snapshot);
            let _ = app.emit("task-update", &snapshot);
        });

        let outcome = if should_share_only {
            outcome_from_history.expect("checked above")
        } else {
            {
                let mut task = task.lock();
                let platform = &mut task.baidu;
                platform.state = PlatformState::Hashing;
                platform.error = None;
                task.updated_at = Utc::now().timestamp();
            }
            self.publish(&task)?;
            let request = UploadRequest {
                task_id: task_id.clone(),
                local_path,
                file_name,
                file_size,
                remote_dir,
                resume_data: existing.resume_data,
                retry_limit: settings.retry_limit,
            };
            match adapter.upload_file(request, progress, cancel.clone()).await {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.finish_with_error(&task, &error)?;
                    self.cancellation.lock().await.remove(&cancel_key);
                    return Err(error);
                }
            }
        };
        if outcome.instant_upload {
            self.log.info("baidu", "官方接口返回秒传或已存在文件");
        }
        {
            let mut task = task.lock();
            let platform = &mut task.baidu;
            platform.remote_path = Some(outcome.remote_path.clone());
            platform.remote_id = outcome.remote_id.clone();
            platform.upload_id = outcome.upload_id.clone();
            platform.resume_data = outcome.resume_data.clone();
            platform.progress = 100.0;
            platform.uploaded_bytes = file_size;
            platform.state = PlatformState::CreatingShare;
            task.updated_at = Utc::now().timestamp();
        }
        self.publish(&task)?;
        match adapter
            .create_share(&outcome, settings.share_period_days)
            .await
        {
            Ok(share) => {
                let mut task = task.lock();
                let platform = &mut task.baidu;
                platform.share = share;
                platform.state = PlatformState::Completed;
                platform.error = None;
                task.updated_at = Utc::now().timestamp();
            }
            Err(error) => {
                self.finish_with_error(&task, &error)?;
                self.cancellation.lock().await.remove(&cancel_key);
                return Err(error);
            }
        }
        self.publish(&task)?;
        self.cancellation.lock().await.remove(&cancel_key);
        Ok(())
    }

    fn finish_with_error(&self, task: &Arc<Mutex<UploadTask>>, error: &AppError) -> AppResult<()> {
        let task_id = task.lock().id.clone();
        let requested_state = self
            .database
            .get_task(&task_id)?
            .map(|persisted| persisted.baidu.state.clone());
        {
            let mut task = task.lock();
            let platform = &mut task.baidu;
            if !matches!(
                platform.state,
                PlatformState::Paused | PlatformState::Cancelled
            ) {
                platform.state = match error {
                    AppError::Paused => match requested_state {
                        Some(PlatformState::Cancelled) => PlatformState::Cancelled,
                        _ => PlatformState::Paused,
                    },
                    _ => PlatformState::Retryable,
                };
            }
            platform.error = Some(error.to_string());
            platform.retry_count += 1;
            task.updated_at = Utc::now().timestamp();
        }
        self.log.error("baidu", &error.to_string());
        self.publish(task).map(|_| ())
    }

    fn publish(&self, task: &Arc<Mutex<UploadTask>>) -> AppResult<UploadTask> {
        let snapshot = task.lock().clone();
        self.database.save_task(&snapshot)?;
        self.emit_task(&snapshot);
        Ok(snapshot)
    }

    fn emit_task(&self, task: &UploadTask) {
        let _ = self.app.emit("task-update", task);
    }
}

fn overall_state(task: &UploadTask) -> TaskState {
    match task.baidu.state {
        PlatformState::Completed => TaskState::Completed,
        PlatformState::Paused => TaskState::Paused,
        PlatformState::Cancelled => TaskState::Cancelled,
        _ => TaskState::Failed,
    }
}

fn duplicate_task(
    batch_id: &str,
    path: &Path,
    file_name: String,
    size: u64,
    hash: String,
    now: i64,
    history: HistoryRecord,
) -> UploadTask {
    let mut baidu = history_platform(
        size,
        history.baidu_path,
        history.baidu_remote_id,
        history.baidu_url,
        history.baidu_code,
        history.baidu_expires_at,
        history.baidu_share_id,
    );
    let needs_upload = baidu.remote_path.is_none();
    let baidu_needs_share = baidu.remote_path.is_some() && !share_valid(&baidu.share, now);
    if needs_upload {
        baidu = PlatformTask::waiting(size);
    }
    if baidu_needs_share {
        baidu.state = PlatformState::CreatingShare;
    }
    let needs_processing = needs_upload || baidu_needs_share;
    UploadTask {
        id: Uuid::new_v4().to_string(),
        batch_id: batch_id.to_string(),
        local_path: path.to_string_lossy().to_string(),
        file_name,
        file_size: size,
        file_hash: hash,
        created_at: now,
        updated_at: now,
        state: if needs_processing {
            TaskState::Waiting
        } else {
            TaskState::Duplicate
        },
        message: Some(if needs_upload {
            "历史记录中没有百度上传结果，将开始上传百度网盘".to_string()
        } else if baidu_needs_share {
            "已发现百度上传记录，将只尝试获取最新分享结果".to_string()
        } else {
            "该文件已经上传过，已直接显示历史分享链接".to_string()
        }),
        baidu,
    }
}

fn history_platform(
    size: u64,
    remote_path: Option<String>,
    remote_id: Option<String>,
    url: Option<String>,
    code: Option<String>,
    expires_at: Option<i64>,
    share_id: Option<String>,
) -> PlatformTask {
    let uploaded = remote_path.is_some();
    PlatformTask {
        state: if uploaded {
            PlatformState::Completed
        } else {
            PlatformState::Unavailable
        },
        progress: if uploaded { 100.0 } else { 0.0 },
        uploaded_bytes: if uploaded { size } else { 0 },
        total_bytes: size,
        remote_path,
        remote_id,
        upload_id: None,
        resume_data: None,
        share: ShareResult {
            url,
            code,
            expires_at,
            share_id,
            remote_path: None,
            notice: None,
        },
        error: None,
        retry_count: 0,
    }
}

fn share_valid(share: &ShareResult, now: i64) -> bool {
    share.url.is_some() && share.expires_at.is_none_or(|expires_at| expires_at > now)
}

fn dated_dir(base: &str, enabled: bool) -> String {
    if enabled {
        format!(
            "{}{}/",
            base.trim_end_matches('/'),
            Local::now().format("/%Y-%m-%d")
        )
    } else {
        base.to_string()
    }
}

fn expand_files(paths: Vec<PathBuf>) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            for entry in WalkDir::new(&path).follow_links(false) {
                let entry = entry.map_err(|error| AppError::File(error.to_string()))?;
                if entry.file_type().is_file() {
                    files.push(entry.into_path());
                }
            }
        } else {
            return Err(AppError::File(format!("路径不存在：{}", path.display())));
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::{dated_dir, duplicate_task, share_valid};
    use crate::models::{HistoryRecord, PlatformState, ShareResult, TaskState};
    use std::path::Path;

    #[test]
    fn expired_share_is_not_reused() {
        let share = ShareResult {
            url: Some("https://example.test".into()),
            expires_at: Some(99),
            ..ShareResult::default()
        };
        assert!(!share_valid(&share, 100));
    }

    #[test]
    fn date_directory_keeps_separator() {
        let value = dated_dir("/apps/demo/", true);
        assert!(value.starts_with("/apps/demo/"));
        assert!(value.ends_with('/'));
    }

    #[test]
    fn duplicate_reuses_valid_baidu_share() {
        let history = HistoryRecord {
            id: "history-1".into(),
            file_name: "file.zip".into(),
            file_size: 10,
            local_path: "C:/file.zip".into(),
            file_hash: "hash".into(),
            uploaded_at: 1,
            task_status: TaskState::Completed,
            baidu_path: Some("/apps/demo/file.zip".into()),
            baidu_remote_id: Some("1".into()),
            baidu_url: Some("https://pan.baidu.com/valid".into()),
            baidu_code: None,
            baidu_expires_at: Some(200),
            baidu_share_id: None,
        };
        let task = duplicate_task(
            "batch",
            Path::new("C:/file.zip"),
            "file.zip".into(),
            10,
            "hash".into(),
            100,
            history,
        );
        assert_eq!(task.state, TaskState::Duplicate);
        assert_eq!(task.baidu.state, PlatformState::Completed);
    }

    #[test]
    fn legacy_history_without_baidu_result_uploads_baidu() {
        let history = HistoryRecord {
            id: "history-legacy".into(),
            file_name: "file.zip".into(),
            file_size: 10,
            local_path: "C:/file.zip".into(),
            file_hash: "hash".into(),
            uploaded_at: 1,
            task_status: TaskState::Completed,
            baidu_path: None,
            baidu_remote_id: None,
            baidu_url: None,
            baidu_code: None,
            baidu_expires_at: None,
            baidu_share_id: None,
        };
        let task = duplicate_task(
            "batch",
            Path::new("C:/file.zip"),
            "file.zip".into(),
            10,
            "hash".into(),
            100,
            history,
        );
        assert_eq!(task.state, TaskState::Waiting);
        assert_eq!(task.baidu.state, PlatformState::Waiting);
    }
}
