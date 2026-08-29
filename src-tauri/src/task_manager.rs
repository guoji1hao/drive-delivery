use crate::{
    adapters::{
        DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest,
        baidu::BaiduDriveAdapter, quark::QuarkDriveAdapter,
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
    quark: Arc<QuarkDriveAdapter>,
    log: Arc<LogService>,
    cancellation: AsyncMutex<HashMap<String, CancellationToken>>,
}

#[derive(Clone, Copy)]
enum Provider {
    Baidu,
    Quark,
}

impl Provider {
    fn name(self) -> &'static str {
        match self {
            Self::Baidu => "baidu",
            Self::Quark => "quark",
        }
    }
}

impl TaskManager {
    pub fn new(
        app: AppHandle,
        database: Arc<Database>,
        settings: Arc<SettingsService>,
        baidu: Arc<BaiduDriveAdapter>,
        quark: Arc<QuarkDriveAdapter>,
        log: Arc<LogService>,
    ) -> Self {
        Self {
            app,
            database,
            settings,
            baidu,
            quark,
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
                    quark: PlatformTask::waiting(size),
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
                        let _ = manager.process_task(&task.id, None).await;
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

    pub async fn pause(&self, task_id: &str, provider: Option<&str>) -> AppResult<UploadTask> {
        self.set_terminal_control_state(task_id, provider, PlatformState::Paused, TaskState::Paused)
            .await
    }

    pub async fn cancel(&self, task_id: &str, provider: Option<&str>) -> AppResult<UploadTask> {
        self.set_terminal_control_state(
            task_id,
            provider,
            PlatformState::Cancelled,
            TaskState::Cancelled,
        )
        .await
    }

    pub async fn retry(
        self: &Arc<Self>,
        task_id: &str,
        provider: Option<&str>,
    ) -> AppResult<UploadTask> {
        let mut task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        for (name, platform) in [("baidu", &mut task.baidu), ("quark", &mut task.quark)] {
            if provider.is_none() || provider == Some(name) {
                if platform.state != PlatformState::Completed {
                    platform.state = PlatformState::Waiting;
                    platform.error = None;
                }
            }
        }
        task.state = TaskState::Waiting;
        task.updated_at = Utc::now().timestamp();
        self.database.save_task(&task)?;
        self.emit_task(&task);
        let manager = Arc::clone(self);
        let task_id = task_id.to_string();
        let provider = provider.map(ToOwned::to_owned);
        tokio::spawn(async move {
            let _ = manager.process_task(&task_id, provider.as_deref()).await;
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
        provider: Option<&str>,
        platform_state: PlatformState,
        task_state: TaskState,
    ) -> AppResult<UploadTask> {
        let mut task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        for (name, platform) in [("baidu", &mut task.baidu), ("quark", &mut task.quark)] {
            if (provider.is_none() || provider == Some(name))
                && platform.state != PlatformState::Completed
            {
                platform.state = platform_state.clone();
            }
        }
        task.state = task_state;
        task.updated_at = Utc::now().timestamp();
        self.database.save_task(&task)?;
        self.emit_task(&task);
        let keys = match provider {
            Some(provider) => vec![format!("{task_id}:{provider}")],
            None => vec![format!("{task_id}:baidu"), format!("{task_id}:quark")],
        };
        let tokens = self.cancellation.lock().await;
        for key in keys {
            if let Some(token) = tokens.get(&key) {
                token.cancel();
            }
        }
        Ok(task)
    }

    async fn process_task(&self, task_id: &str, only: Option<&str>) -> AppResult<()> {
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
        let baidu_enabled = only.is_none() || only == Some("baidu");
        let quark_enabled = only.is_none() || only == Some("quark");
        let baidu_future = async {
            if baidu_enabled {
                self.process_provider_with_retries(Arc::clone(&shared), Provider::Baidu, &settings)
                    .await
            } else {
                Ok(())
            }
        };
        let quark_future = async {
            if quark_enabled {
                self.process_provider_with_retries(Arc::clone(&shared), Provider::Quark, &settings)
                    .await
            } else {
                Ok(())
            }
        };
        let _ = tokio::join!(baidu_future, quark_future);
        {
            let mut task = shared.lock();
            task.state = overall_state(&task);
            task.updated_at = Utc::now().timestamp();
            if task.state == TaskState::Completed {
                task.message = Some("两个平台均已处理完成".to_string());
            } else if task.state == TaskState::PartialSuccess {
                task.message = Some("部分平台已完成，可单独重试失败平台".to_string());
            }
        }
        let final_task = self.publish(&shared)?;
        if matches!(
            final_task.state,
            TaskState::Completed | TaskState::PartialSuccess | TaskState::Duplicate
        ) {
            self.database.save_history(&final_task)?;
            if settings.auto_copy {
                let text = share_manager::format_task(&final_task, &settings, None);
                if text.contains("http") {
                    let _ = clipboard::copy_text(&text);
                }
            }
        }
        Ok(())
    }

    async fn process_provider_with_retries(
        &self,
        task: Arc<Mutex<UploadTask>>,
        provider: Provider,
        settings: &AppSettings,
    ) -> AppResult<()> {
        let mut retry = 0_u32;
        loop {
            match self
                .process_provider(Arc::clone(&task), provider, settings)
                .await
            {
                Ok(()) => return Ok(()),
                Err(error)
                    if matches!(error, AppError::Provider(_)) && retry < settings.retry_limit =>
                {
                    retry += 1;
                    let delay = 1_u64 << (retry - 1).min(5);
                    self.log.info(
                        provider.name(),
                        &format!("平台任务失败，{delay} 秒后自动执行第 {retry} 次重试"),
                    );
                    sleep(Duration::from_secs(delay)).await;

                    let task_id = task.lock().id.clone();
                    if let Some(persisted) = self.database.get_task(&task_id)? {
                        let requested = platform_ref(&persisted, provider).state.clone();
                        if matches!(requested, PlatformState::Paused | PlatformState::Cancelled) {
                            let mut task = task.lock();
                            platform_mut(&mut task, provider).state = requested;
                            return Err(AppError::Paused);
                        }
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn process_provider(
        &self,
        task: Arc<Mutex<UploadTask>>,
        provider: Provider,
        settings: &AppSettings,
    ) -> AppResult<()> {
        let (task_id, local_path, file_name, file_size, existing, previous_state) = {
            let task = task.lock();
            let platform = platform_ref(&task, provider);
            (
                task.id.clone(),
                PathBuf::from(&task.local_path),
                task.file_name.clone(),
                task.file_size,
                platform.clone(),
                platform.state.clone(),
            )
        };
        if previous_state == PlatformState::Completed {
            return Ok(());
        }
        let adapter: Arc<dyn DriveAdapter> = match provider {
            Provider::Baidu => self.baidu.clone(),
            Provider::Quark => self.quark.clone(),
        };
        let cancel = CancellationToken::new();
        let cancel_key = format!("{}:{}", task_id, provider.name());
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
        let remote_dir = dated_dir(
            match provider {
                Provider::Baidu => &settings.baidu_default_dir,
                Provider::Quark => &settings.quark_default_dir,
            },
            settings.dated_subdirectory,
        );
        let progress_task = Arc::clone(&task);
        let database = Arc::clone(&self.database);
        let app = self.app.clone();
        let progress_provider = provider;
        let progress: ProgressSink = Arc::new(move |value: UploadProgress| {
            let snapshot = {
                let mut task = progress_task.lock();
                let platform = platform_mut(&mut task, progress_provider);
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
                let platform = platform_mut(&mut task, provider);
                platform.state = if matches!(provider, Provider::Baidu) {
                    PlatformState::Hashing
                } else {
                    PlatformState::Uploading
                };
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
                    self.finish_with_error(&task, provider, &error)?;
                    self.cancellation.lock().await.remove(&cancel_key);
                    return Err(error);
                }
            }
        };
        if outcome.instant_upload {
            self.log
                .info(provider.name(), "官方接口返回秒传或已存在文件");
        }
        {
            let mut task = task.lock();
            let platform = platform_mut(&mut task, provider);
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
                let platform = platform_mut(&mut task, provider);
                platform.share = share;
                platform.state = PlatformState::Completed;
                platform.error = None;
                task.updated_at = Utc::now().timestamp();
            }
            Err(error) => {
                self.finish_with_error(&task, provider, &error)?;
                self.cancellation.lock().await.remove(&cancel_key);
                return Err(error);
            }
        }
        self.publish(&task)?;
        self.cancellation.lock().await.remove(&cancel_key);
        Ok(())
    }

    fn finish_with_error(
        &self,
        task: &Arc<Mutex<UploadTask>>,
        provider: Provider,
        error: &AppError,
    ) -> AppResult<()> {
        let task_id = task.lock().id.clone();
        let requested_state = self
            .database
            .get_task(&task_id)?
            .map(|persisted| platform_ref(&persisted, provider).state.clone());
        {
            let mut task = task.lock();
            let platform = platform_mut(&mut task, provider);
            if !matches!(
                platform.state,
                PlatformState::Paused | PlatformState::Cancelled
            ) {
                platform.state = match error {
                    AppError::Unavailable(_) => PlatformState::Unavailable,
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
        self.log.error(provider.name(), &error.to_string());
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

fn platform_ref(task: &UploadTask, provider: Provider) -> &PlatformTask {
    match provider {
        Provider::Baidu => &task.baidu,
        Provider::Quark => &task.quark,
    }
}

fn platform_mut(task: &mut UploadTask, provider: Provider) -> &mut PlatformTask {
    match provider {
        Provider::Baidu => &mut task.baidu,
        Provider::Quark => &mut task.quark,
    }
}

fn overall_state(task: &UploadTask) -> TaskState {
    let states = [&task.baidu.state, &task.quark.state];
    if states
        .iter()
        .all(|state| **state == PlatformState::Completed)
    {
        TaskState::Completed
    } else if states
        .iter()
        .any(|state| **state == PlatformState::Completed)
    {
        TaskState::PartialSuccess
    } else if states.iter().any(|state| **state == PlatformState::Paused) {
        TaskState::Paused
    } else if states
        .iter()
        .all(|state| **state == PlatformState::Cancelled)
    {
        TaskState::Cancelled
    } else {
        TaskState::Failed
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
    let mut quark = history_platform(
        size,
        history.quark_path,
        history.quark_remote_id,
        history.quark_url,
        history.quark_code,
        history.quark_expires_at,
        history.quark_share_id,
    );
    let baidu_needs_share = baidu.remote_path.is_some() && !share_valid(&baidu.share, now);
    let quark_needs_share = quark.remote_path.is_some() && !share_valid(&quark.share, now);
    if baidu_needs_share {
        baidu.state = PlatformState::CreatingShare;
    }
    if quark_needs_share {
        quark.state = PlatformState::CreatingShare;
    }
    let needs_share = baidu_needs_share || quark_needs_share;
    UploadTask {
        id: Uuid::new_v4().to_string(),
        batch_id: batch_id.to_string(),
        local_path: path.to_string_lossy().to_string(),
        file_name,
        file_size: size,
        file_hash: hash,
        created_at: now,
        updated_at: now,
        state: if needs_share {
            TaskState::Waiting
        } else {
            TaskState::Duplicate
        },
        message: Some(if needs_share {
            "历史分享已过期，将只尝试重新创建分享".to_string()
        } else {
            "该文件已经上传过，已直接显示历史分享链接".to_string()
        }),
        baidu,
        quark,
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
    fn duplicate_retries_only_the_expired_platform_share() {
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
            quark_path: Some("夸克网盘/file.zip".into()),
            quark_remote_id: Some("fid".into()),
            quark_url: Some("https://pan.quark.cn/expired".into()),
            quark_code: None,
            quark_expires_at: Some(99),
            quark_share_id: None,
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
        assert_eq!(task.baidu.state, PlatformState::Completed);
        assert_eq!(task.quark.state, PlatformState::CreatingShare);
    }
}
