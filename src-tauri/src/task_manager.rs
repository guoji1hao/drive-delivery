use crate::{
    adapters::{DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest},
    clipboard,
    database::Database,
    errors::{AppError, AppResult},
    logging::LogService,
    models::{
        AppSettings, EnqueueResult, HistoryRecord, PlatformState, PlatformTask, ShareResult,
        TaskState, UploadTask,
    },
    plugins::PluginService,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Baidu,
    Google,
    OneDrive,
}

impl Provider {
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "baidu" => Ok(Self::Baidu),
            "google" => Ok(Self::Google),
            "onedrive" => Ok(Self::OneDrive),
            _ => Err(AppError::Configuration(format!("不支持的平台：{value}"))),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Baidu => "baidu",
            Self::Google => "google",
            Self::OneDrive => "onedrive",
        }
    }
}

pub struct TaskManager {
    app: AppHandle,
    database: Arc<Database>,
    settings: Arc<SettingsService>,
    plugins: Arc<PluginService>,
    log: Arc<LogService>,
    cancellation: AsyncMutex<HashMap<String, CancellationToken>>,
}

impl TaskManager {
    pub fn new(
        app: AppHandle,
        database: Arc<Database>,
        settings: Arc<SettingsService>,
        plugins: Arc<PluginService>,
        log: Arc<LogService>,
    ) -> Self {
        Self {
            app,
            database,
            settings,
            plugins,
            log,
            cancellation: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn enqueue_paths(self: &Arc<Self>, paths: Vec<String>) -> AppResult<EnqueueResult> {
        let enabled: Vec<_> = [Provider::Baidu, Provider::Google, Provider::OneDrive]
            .into_iter()
            .filter(|p| self.plugins.enabled(p.key()))
            .collect();
        if enabled.is_empty() {
            return Err(AppError::Configuration(
                "请先在网盘插件中启用至少一个网盘".into(),
            ));
        }
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
            let mut task = if let Some(history) = history {
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
                    google: PlatformTask::waiting(size),
                    onedrive: PlatformTask::waiting(size),
                }
            };
            select_platforms(&mut task, &enabled);
            let should_start = !matches!(task.state, TaskState::Duplicate | TaskState::Completed);
            self.database.save_task(&task)?;
            self.emit_task(&task);
            tasks.push(task.clone());
            if should_start {
                let manager = Arc::clone(self);
                let semaphore = Arc::clone(&semaphore);
                tokio::spawn(async move {
                    if let Ok(_permit) = semaphore.acquire_owned().await {
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

    pub async fn pause(&self, task_id: &str, provider: Provider) -> AppResult<UploadTask> {
        self.set_control_state(task_id, provider, PlatformState::Paused)
            .await
    }

    pub async fn cancel(&self, task_id: &str, provider: Provider) -> AppResult<UploadTask> {
        self.set_control_state(task_id, provider, PlatformState::Cancelled)
            .await
    }

    pub async fn retry(
        self: &Arc<Self>,
        task_id: &str,
        provider: Provider,
    ) -> AppResult<UploadTask> {
        if !self.plugins.enabled(provider.key()) {
            return Err(AppError::Configuration(
                "请先启用该网盘插件，再重试任务".into(),
            ));
        }
        let mut task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        let target = platform_mut(&mut task, provider);
        if target.state != PlatformState::Completed {
            target.state = PlatformState::Waiting;
            target.error = None;
        }
        task.state = TaskState::Processing;
        task.updated_at = Utc::now().timestamp();
        self.database.save_task(&task)?;
        self.emit_task(&task);
        let manager = Arc::clone(self);
        let task_id = task_id.to_string();
        tokio::spawn(async move {
            let _ = manager.process_task(&task_id, Some(provider)).await;
        });
        Ok(task)
    }

    pub fn list_tasks(&self) -> AppResult<Vec<UploadTask>> {
        self.database.list_tasks(500)
    }

    pub fn search_history(&self, query: &str) -> AppResult<Vec<HistoryRecord>> {
        self.database.search_history(query, 500)
    }

    async fn set_control_state(
        &self,
        task_id: &str,
        provider: Provider,
        state: PlatformState,
    ) -> AppResult<UploadTask> {
        let mut task = self
            .database
            .get_task(task_id)?
            .ok_or_else(|| AppError::NotFound(task_id.to_string()))?;
        if platform(&task, provider).state != PlatformState::Completed {
            platform_mut(&mut task, provider).state = state;
        }
        task.state = overall_state(&task);
        task.updated_at = Utc::now().timestamp();
        self.database.save_task(&task)?;
        self.emit_task(&task);
        if let Some(token) = self
            .cancellation
            .lock()
            .await
            .get(&cancellation_key(task_id, provider))
        {
            token.cancel();
        }
        Ok(task)
    }

    async fn process_task(&self, task_id: &str, only: Option<Provider>) -> AppResult<()> {
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

        match only {
            Some(provider) => {
                let _ = self
                    .process_provider_with_retries(Arc::clone(&shared), &settings, provider)
                    .await;
            }
            None => {
                let _ = tokio::join!(
                    self.process_provider_with_retries(
                        Arc::clone(&shared),
                        &settings,
                        Provider::Baidu
                    ),
                    self.process_provider_with_retries(
                        Arc::clone(&shared),
                        &settings,
                        Provider::Google
                    ),
                    self.process_provider_with_retries(
                        Arc::clone(&shared),
                        &settings,
                        Provider::OneDrive
                    )
                );
            }
        }

        {
            let mut task = shared.lock();
            task.state = overall_state(&task);
            task.updated_at = Utc::now().timestamp();
            task.message = Some(match task.state {
                TaskState::Completed => "所有平台均处理完成".to_string(),
                TaskState::PartialSuccess => "部分平台已完成，其余平台可单独重试".to_string(),
                _ => "平台处理未全部完成，可按平台重试".to_string(),
            });
        }
        let final_task = self.publish(&shared)?;
        if matches!(
            final_task.state,
            TaskState::Completed | TaskState::PartialSuccess | TaskState::Duplicate
        ) {
            self.database.save_history(&final_task)?;
            if settings.auto_copy {
                let text = share_manager::format_task(
                    &final_task,
                    &settings,
                    self.database
                        .get_metadata("ui_language")?
                        .as_deref()
                        .unwrap_or("zh-CN"),
                );
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
        settings: &AppSettings,
        provider: Provider,
    ) -> AppResult<()> {
        let mut retry = 0_u32;
        loop {
            match self
                .process_provider(Arc::clone(&task), settings, provider)
                .await
            {
                Ok(()) => return Ok(()),
                Err(error)
                    if matches!(error, AppError::Provider(_)) && retry < settings.retry_limit =>
                {
                    retry += 1;
                    let delay = 1_u64 << (retry - 1).min(5);
                    self.log.info(
                        provider.key(),
                        &format!("平台任务失败，{delay} 秒后执行第 {retry} 次有限重试"),
                    );
                    sleep(Duration::from_secs(delay)).await;
                    let task_id = task.lock().id.clone();
                    if let Some(persisted) = self.database.get_task(&task_id)? {
                        let requested = platform(&persisted, provider).state.clone();
                        if matches!(requested, PlatformState::Paused | PlatformState::Cancelled) {
                            let mut shared = task.lock();
                            platform_mut(&mut shared, provider).state = requested;
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
        settings: &AppSettings,
        provider: Provider,
    ) -> AppResult<()> {
        let (task_id, local_path, file_name, file_size, existing, previous_state) = {
            let task = task.lock();
            let platform = platform(&task, provider);
            (
                task.id.clone(),
                PathBuf::from(&task.local_path),
                task.file_name.clone(),
                task.file_size,
                platform.clone(),
                platform.state.clone(),
            )
        };
        if matches!(
            previous_state,
            PlatformState::Completed | PlatformState::Skipped
        ) {
            return Ok(());
        }
        if !self.plugins.enabled(provider.key()) {
            platform_mut(&mut task.lock(), provider).state = PlatformState::Skipped;
            self.publish(&task)?;
            return Ok(());
        }
        let adapter: Arc<dyn DriveAdapter> = self.plugins.adapter(provider.key())?;
        let cancel = CancellationToken::new();
        let cancel_key = cancellation_key(&task_id, provider);
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
        let should_share_only = existing.remote_id.is_some() && outcome_from_history.is_some();
        let base_dir = match provider {
            Provider::Baidu => &settings.baidu_default_dir,
            Provider::Google => &settings.google_default_dir,
            Provider::OneDrive => &settings.onedrive_default_dir,
        };
        let remote_dir = dated_dir(base_dir, settings.dated_subdirectory);
        let progress_task = Arc::clone(&task);
        let database = Arc::clone(&self.database);
        let app = self.app.clone();
        let progress: ProgressSink = Arc::new(move |value: UploadProgress| {
            let snapshot = {
                let mut task = progress_task.lock();
                let platform = platform_mut(&mut task, provider);
                platform.state = PlatformState::Uploading;
                platform.uploaded_bytes = value.uploaded_bytes;
                if value.total_bytes > 0 {
                    platform.total_bytes = value.total_bytes;
                }
                platform.progress = value.percent.clamp(0.0, 100.0);
                if value.upload_id.is_some() {
                    platform.upload_id = value.upload_id;
                }
                if value.resume_data.is_some() || value.percent >= 100.0 {
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
                    self.finish_with_error(&task, provider, &error)?;
                    self.cancellation.lock().await.remove(&cancel_key);
                    return Err(error);
                }
            }
        };
        if outcome.instant_upload {
            self.log.info(provider.key(), "复用了已存在的远端上传结果");
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
            .map(|persisted| platform(&persisted, provider).state.clone());
        {
            let mut task = task.lock();
            let platform = platform_mut(&mut task, provider);
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
        self.log.error(provider.key(), &error.to_string());
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

fn platform(task: &UploadTask, provider: Provider) -> &PlatformTask {
    match provider {
        Provider::Baidu => &task.baidu,
        Provider::Google => &task.google,
        Provider::OneDrive => &task.onedrive,
    }
}

fn platform_mut(task: &mut UploadTask, provider: Provider) -> &mut PlatformTask {
    match provider {
        Provider::Baidu => &mut task.baidu,
        Provider::Google => &mut task.google,
        Provider::OneDrive => &mut task.onedrive,
    }
}

fn cancellation_key(task_id: &str, provider: Provider) -> String {
    format!("{task_id}:{}", provider.key())
}

fn select_platforms(task: &mut UploadTask, enabled: &[Provider]) {
    for provider in [Provider::Baidu, Provider::Google, Provider::OneDrive] {
        let target = platform_mut(task, provider);
        if !enabled.contains(&provider) && target.state != PlatformState::Completed {
            target.state = PlatformState::Skipped;
        }
    }
    if task.state != TaskState::Duplicate {
        task.state = overall_state(task);
    }
}

fn overall_state(task: &UploadTask) -> TaskState {
    let states: Vec<_> = [&task.baidu.state, &task.google.state, &task.onedrive.state]
        .into_iter()
        .filter(|state| **state != PlatformState::Skipped)
        .collect();
    if states.is_empty() {
        return TaskState::Cancelled;
    }
    let completed = states
        .iter()
        .filter(|state| **state == &PlatformState::Completed)
        .count();
    if completed == states.len() {
        return TaskState::Completed;
    }
    if completed > 0 {
        return TaskState::PartialSuccess;
    }
    if states.iter().any(|state| {
        matches!(
            **state,
            PlatformState::Hashing | PlatformState::Uploading | PlatformState::CreatingShare
        )
    }) {
        return TaskState::Processing;
    }
    if states.iter().all(|state| **state == PlatformState::Waiting) {
        return TaskState::Waiting;
    }
    if states.iter().all(|state| **state == PlatformState::Paused) {
        return TaskState::Paused;
    }
    if states
        .iter()
        .all(|state| **state == PlatformState::Cancelled)
    {
        return TaskState::Cancelled;
    }
    TaskState::Failed
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
    let mut google = history_platform(
        size,
        history.google_path,
        history.google_remote_id,
        history.google_url,
        history.google_code,
        history.google_expires_at,
        history.google_share_id,
    );
    let mut onedrive = history_platform(
        size,
        history.onedrive_path,
        history.onedrive_remote_id,
        history.onedrive_url,
        history.onedrive_code,
        history.onedrive_expires_at,
        history.onedrive_share_id,
    );
    let baidu_needs_upload = baidu.remote_path.is_none();
    let google_needs_upload = google.remote_path.is_none();
    let onedrive_needs_upload = onedrive.remote_path.is_none();
    let baidu_needs_share = baidu.remote_path.is_some() && !share_valid(&baidu.share, now);
    let google_needs_share = google.remote_path.is_some() && !share_valid(&google.share, now);
    let onedrive_needs_share = onedrive.remote_path.is_some() && !share_valid(&onedrive.share, now);
    if baidu_needs_upload {
        baidu = PlatformTask::waiting(size);
    } else if baidu_needs_share {
        baidu.state = PlatformState::CreatingShare;
    }
    if google_needs_upload {
        google = PlatformTask::waiting(size);
    } else if google_needs_share {
        google.state = PlatformState::CreatingShare;
    }
    if onedrive_needs_upload {
        onedrive = PlatformTask::waiting(size);
    } else if onedrive_needs_share {
        onedrive.state = PlatformState::CreatingShare;
    }
    let needs_processing = baidu_needs_upload
        || google_needs_upload
        || baidu_needs_share
        || google_needs_share
        || onedrive_needs_upload
        || onedrive_needs_share;
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
        message: Some(
            if baidu_needs_upload || google_needs_upload || onedrive_needs_upload {
                "已命中历史记录，仅上传缺失的平台".to_string()
            } else if baidu_needs_share || google_needs_share || onedrive_needs_share {
                "已命中上传记录，仅重新创建失效的分享".to_string()
            } else {
                "该文件已经上传过，已直接显示历史分享结果".to_string()
            },
        ),
        baidu,
        google,
        onedrive,
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
        remote_path: remote_path.clone(),
        remote_id,
        upload_id: None,
        resume_data: None,
        share: ShareResult {
            url,
            code,
            expires_at,
            share_id,
            remote_path,
            notice: None,
        },
        error: None,
        retry_count: 0,
    }
}

fn share_valid(share: &ShareResult, now: i64) -> bool {
    (share.url.is_some() || share.notice.is_some())
        && share.expires_at.is_none_or(|expires_at| expires_at > now)
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
    use super::{Provider, duplicate_task, overall_state, share_valid};
    use crate::models::{HistoryRecord, PlatformState, PlatformTask, ShareResult, TaskState};
    use std::path::Path;

    fn history() -> HistoryRecord {
        HistoryRecord {
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
            baidu_expires_at: None,
            baidu_share_id: None,
            google_path: Some("My Drive/自动发货/file.zip".into()),
            onedrive_path: Some("OneDrive/自动发货/file.zip".into()),
            google_remote_id: Some("google-file".into()),
            onedrive_remote_id: Some("onedrive-file".into()),
            google_url: Some("https://drive.google.com/file/d/demo/view".into()),
            onedrive_url: Some("https://example.test/onedrive-share".into()),
            google_code: None,
            onedrive_code: None,
            google_expires_at: None,
            onedrive_expires_at: None,
            google_share_id: Some("anyoneWithLink".into()),
            onedrive_share_id: Some("anyoneWithLink".into()),
        }
    }

    #[test]
    fn disabled_missing_drives_do_not_prevent_completion_or_lose_history() {
        let mut record = history();
        record.baidu_path = None;
        record.baidu_url = None;
        record.onedrive_path = None;
        record.onedrive_url = None;
        let mut task = duplicate_task(
            "batch",
            Path::new("file.zip"),
            "file.zip".into(),
            10,
            "hash".into(),
            100,
            record,
        );
        super::select_platforms(&mut task, &[Provider::Google]);
        assert_eq!(task.baidu.state, PlatformState::Skipped);
        assert_eq!(task.onedrive.state, PlatformState::Skipped);
        assert_eq!(task.state, TaskState::Completed);
        assert!(task.google.share.url.is_some());
        task.google.state = PlatformState::Skipped;
        assert_eq!(overall_state(&task), TaskState::Cancelled);
    }

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
    fn duplicate_reuses_both_valid_results() {
        let task = duplicate_task(
            "batch",
            Path::new("C:/file.zip"),
            "file.zip".into(),
            10,
            "hash".into(),
            100,
            history(),
        );
        assert_eq!(task.state, TaskState::Duplicate);
        assert_eq!(task.google.state, PlatformState::Completed);
    }

    #[test]
    fn missing_google_result_only_schedules_google() {
        let mut history = history();
        history.google_path = None;
        history.google_remote_id = None;
        history.google_url = None;
        history.google_share_id = None;
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
        assert_eq!(task.google.state, PlatformState::Waiting);
    }

    #[test]
    fn onedrive_missing_or_expired_preserves_other_platforms() {
        let mut record = history();
        record.onedrive_expires_at = Some(90);
        let task = duplicate_task(
            "batch",
            Path::new("file.zip"),
            "file.zip".into(),
            10,
            "hash".into(),
            100,
            record.clone(),
        );
        assert_eq!(task.onedrive.state, PlatformState::CreatingShare);
        assert_eq!(task.google.state, PlatformState::Completed);
        assert_eq!(task.baidu.state, PlatformState::Completed);
        record.onedrive_path = None;
        record.onedrive_remote_id = None;
        record.onedrive_url = None;
        let task = duplicate_task(
            "batch",
            Path::new("file.zip"),
            "file.zip".into(),
            10,
            "hash".into(),
            100,
            record,
        );
        assert_eq!(task.onedrive.state, PlatformState::Waiting);
        assert_eq!(task.google.state, PlatformState::Completed);
        assert_eq!(Provider::parse("onedrive").unwrap(), Provider::OneDrive);
    }

    #[test]
    fn one_success_is_partial_success() {
        let task = crate::models::UploadTask {
            id: "1".into(),
            batch_id: "1".into(),
            local_path: "x".into(),
            file_name: "x".into(),
            file_size: 1,
            file_hash: "x".into(),
            created_at: 1,
            updated_at: 1,
            state: TaskState::Processing,
            message: None,
            baidu: PlatformTask {
                state: PlatformState::Completed,
                ..PlatformTask::waiting(1)
            },
            onedrive: PlatformTask::waiting(1),
            google: PlatformTask {
                state: PlatformState::Retryable,
                ..PlatformTask::waiting(1)
            },
        };
        assert_eq!(overall_state(&task), TaskState::PartialSuccess);
        assert_eq!(Provider::parse("google").unwrap(), Provider::Google);
    }
}
