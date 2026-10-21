use crate::{
    clipboard,
    errors::AppResult,
    models::{
        AppSettings, AppSnapshot, AuthChallenge, AuthPollResult, AuthStatus, CapabilityReport,
        EnqueueResult, GoogleCredentialsInput, HistoryRecord, ProviderCredentialsInput, UploadTask,
    },
    share_manager,
    state::AppState,
    task_manager::Provider,
};
use tauri::State;

#[tauri::command]
pub async fn initialize_app(state: State<'_, AppState>) -> AppResult<AppSnapshot> {
    Ok(AppSnapshot {
        plugins: state.plugins.list(),
        tasks: state.tasks.list_tasks()?,
        settings: state.settings.get()?,
        auth: vec![
            state.auth.baidu_status(),
            state.auth.google_status(),
            state.auth.onedrive_status(),
        ],
        capabilities: capability_reports(),
        data_dir: state.data_dir.to_string_lossy().to_string(),
        log_dir: state.log.dir().to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn update_drive_plugin(
    state: State<'_, AppState>,
    id: String,
    action: String,
    manifest: Option<String>,
) -> AppResult<Vec<crate::plugins::DrivePlugin>> {
    state.plugins.update(&id, &action, manifest.as_deref())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> AppResult<AppSettings> {
    state.settings.save(settings)
}

#[tauri::command]
pub fn save_baidu_credentials(
    state: State<'_, AppState>,
    credentials: ProviderCredentialsInput,
) -> AppResult<AuthStatus> {
    state.auth.save_baidu_credentials(credentials)?;
    Ok(state.auth.baidu_status())
}

#[tauri::command]
pub async fn start_baidu_auth(state: State<'_, AppState>) -> AppResult<AuthChallenge> {
    state.auth.start_baidu_auth().await
}

#[tauri::command]
pub async fn poll_baidu_auth(
    state: State<'_, AppState>,
    device_code: String,
) -> AppResult<AuthPollResult> {
    state.auth.poll_baidu_auth(&device_code).await
}

#[tauri::command]
pub fn clear_baidu_auth(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.clear_baidu_auth()?;
    Ok(state.auth.baidu_status())
}

#[tauri::command]
pub fn save_google_credentials(
    state: State<'_, AppState>,
    credentials: GoogleCredentialsInput,
) -> AppResult<AuthStatus> {
    state.auth.save_google_credentials(credentials)?;
    Ok(state.auth.google_status())
}

#[tauri::command]
pub async fn start_google_auth(state: State<'_, AppState>) -> AppResult<AuthChallenge> {
    state.auth.start_google_auth().await
}

#[tauri::command]
pub async fn poll_google_auth(
    state: State<'_, AppState>,
    device_code: String,
) -> AppResult<AuthPollResult> {
    state.auth.poll_google_auth(&device_code).await
}

#[tauri::command]
pub fn clear_google_auth(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.clear_google_auth()?;
    Ok(state.auth.google_status())
}

#[tauri::command]
pub async fn save_onedrive_credentials(
    state: State<'_, AppState>,
    client_id: String,
) -> AppResult<AuthStatus> {
    state.auth.save_onedrive_credentials(&client_id).await?;
    Ok(state.auth.onedrive_status())
}

#[tauri::command]
pub async fn start_onedrive_auth(state: State<'_, AppState>) -> AppResult<AuthChallenge> {
    state.auth.start_onedrive_auth().await
}

#[tauri::command]
pub async fn poll_onedrive_auth(
    state: State<'_, AppState>,
    device_code: String,
) -> AppResult<AuthPollResult> {
    state.auth.poll_onedrive_auth(&device_code).await
}

#[tauri::command]
pub async fn clear_onedrive_auth(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.clear_onedrive_auth().await?;
    Ok(state.auth.onedrive_status())
}

#[tauri::command]
pub async fn enqueue_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> AppResult<EnqueueResult> {
    state.tasks.enqueue_paths(paths).await
}

#[tauri::command]
pub async fn pause_task(
    state: State<'_, AppState>,
    task_id: String,
    provider: String,
) -> AppResult<UploadTask> {
    state
        .tasks
        .pause(&task_id, Provider::parse(&provider)?)
        .await
}

#[tauri::command]
pub async fn cancel_task(
    state: State<'_, AppState>,
    task_id: String,
    provider: String,
) -> AppResult<UploadTask> {
    state
        .tasks
        .cancel(&task_id, Provider::parse(&provider)?)
        .await
}

#[tauri::command]
pub async fn retry_task(
    state: State<'_, AppState>,
    task_id: String,
    provider: String,
) -> AppResult<UploadTask> {
    state
        .tasks
        .retry(&task_id, Provider::parse(&provider)?)
        .await
}

#[tauri::command]
pub fn search_history(state: State<'_, AppState>, query: String) -> AppResult<Vec<HistoryRecord>> {
    state.tasks.search_history(&query)
}

#[tauri::command]
pub fn copy_task_result(
    state: State<'_, AppState>,
    task_id: String,
    provider: Option<String>,
    language: Option<String>,
) -> AppResult<String> {
    let task = state
        .database
        .get_task(&task_id)?
        .ok_or_else(|| crate::errors::AppError::NotFound(task_id.clone()))?;
    let text = share_manager::format_task_for(
        &task,
        &state.settings.get()?,
        provider.as_deref(),
        language.as_deref().unwrap_or("zh-CN"),
    )?;
    clipboard::copy_text(&text)?;
    Ok(text)
}

#[tauri::command]
pub fn copy_history_result(
    state: State<'_, AppState>,
    record: HistoryRecord,
    provider: Option<String>,
    language: Option<String>,
) -> AppResult<String> {
    let text = share_manager::format_history_for(
        &record,
        &state.settings.get()?,
        provider.as_deref(),
        language.as_deref().unwrap_or("zh-CN"),
    )?;
    clipboard::copy_text(&text)?;
    Ok(text)
}

fn capability_reports() -> Vec<CapabilityReport> {
    vec![
        CapabilityReport {
            provider: "onedrive".into(),
            oauth: "Microsoft 官方设备码授权与自动刷新".into(),
            upload: "Microsoft Graph，5 MiB 流式分片".into(),
            resume: "GET uploadUrl 查询 nextExpectedRanges".into(),
            share: "createLink 创建只读公开链接".into(),
            limitations: vec![
                "需要注册公共客户端并授权 Files.ReadWrite / offline_access".into(),
                "组织策略可能禁止匿名分享；有效期以服务端返回值为准".into(),
                "文件名追加任务 ID，防止重试覆盖已有文件".into(),
            ],
        },
        CapabilityReport {
            provider: "baidu".to_string(),
            oauth: "官方 OAuth 2.0 设备码授权与 Refresh Token".to_string(),
            upload: "官方三步上传：precreate、4 MiB 分片、create；支持秒传".to_string(),
            resume: "保存 uploadid、缺失分片和已完成分片，分片边界续传".to_string(),
            share: "当前未确认个人开放 API 支持自动创建公开分享".to_string(),
            limitations: vec![
                "上传目录必须位于开放平台分配的 /apps/应用目录/ 下".to_string(),
                "公开分享能力在新版平台中面向企业方案，个人开放接口未确认".to_string(),
            ],
        },
        CapabilityReport {
            provider: "google".to_string(),
            oauth: "官方桌面 OAuth 2.0：系统浏览器、随机本机回调、PKCE 与 Refresh Token"
                .to_string(),
            upload: "Drive API v3 resumable upload，8 MiB 分片流式读取".to_string(),
            resume: "持久化 session URI，并以 308 Range 查询服务器真实断点".to_string(),
            share: "permissions.create 创建 anyone/reader，读取 webViewLink".to_string(),
            limitations: vec![
                "仅申请 drive.file 最小权限，只管理本应用创建或用户授权给本应用的文件".to_string(),
                "公开链接无提取码；anyone 权限不支持设置 expirationTime".to_string(),
                "Google Workspace 管理员策略可能禁止公开分享".to_string(),
            ],
        },
    ]
}

#[tauri::command]
pub fn save_ui_language(state: State<'_, AppState>, language: String) -> AppResult<()> {
    if !["zh-CN", "en"].contains(&language.as_str()) {
        return Err(crate::errors::AppError::Configuration(
            "Unsupported language".into(),
        ));
    }
    state.database.set_metadata("ui_language", &language)
}
