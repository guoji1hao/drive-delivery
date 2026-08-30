use crate::{
    adapters::DriveAdapter,
    clipboard,
    errors::AppResult,
    models::{
        AppSettings, AppSnapshot, AuthChallenge, AuthPollResult, AuthStatus, CapabilityReport,
        EnqueueResult, HistoryRecord, ProviderCredentialsInput, UploadTask,
    },
    share_manager,
    state::AppState,
};
use tauri::State;

#[tauri::command]
pub async fn initialize_app(state: State<'_, AppState>) -> AppResult<AppSnapshot> {
    let quark_status = state.quark.authenticate().await.unwrap_or(AuthStatus {
        provider: "quark".to_string(),
        authenticated: false,
        available: false,
        expires_at: None,
        message: "夸克官方组件状态检查失败".to_string(),
    });
    Ok(AppSnapshot {
        tasks: state.tasks.list_tasks()?,
        settings: state.settings.get()?,
        auth: vec![state.auth.baidu_status(), quark_status],
        capabilities: capability_reports(),
        data_dir: state.data_dir.to_string_lossy().to_string(),
        log_dir: state.log.dir().to_string_lossy().to_string(),
    })
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
pub async fn start_quark_auth(state: State<'_, AppState>) -> AppResult<String> {
    state.quark.login().await
}

#[tauri::command]
pub async fn install_and_start_quark_auth(state: State<'_, AppState>) -> AppResult<String> {
    state.quark.install_official_component().await?;
    state.quark.login().await
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
    provider: Option<String>,
) -> AppResult<UploadTask> {
    state.tasks.pause(&task_id, provider.as_deref()).await
}

#[tauri::command]
pub async fn cancel_task(
    state: State<'_, AppState>,
    task_id: String,
    provider: Option<String>,
) -> AppResult<UploadTask> {
    state.tasks.cancel(&task_id, provider.as_deref()).await
}

#[tauri::command]
pub async fn retry_task(
    state: State<'_, AppState>,
    task_id: String,
    provider: Option<String>,
) -> AppResult<UploadTask> {
    state.tasks.retry(&task_id, provider.as_deref()).await
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
) -> AppResult<String> {
    let task = state
        .database
        .get_task(&task_id)?
        .ok_or_else(|| crate::errors::AppError::NotFound(task_id.clone()))?;
    let text = share_manager::format_task(&task, &state.settings.get()?, provider.as_deref());
    clipboard::copy_text(&text)?;
    Ok(text)
}

#[tauri::command]
pub fn copy_history_result(state: State<'_, AppState>, record: HistoryRecord) -> AppResult<String> {
    let text = share_manager::format_history(&record, &state.settings.get()?);
    clipboard::copy_text(&text)?;
    Ok(text)
}

fn capability_reports() -> Vec<CapabilityReport> {
    vec![
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
            provider: "quark".to_string(),
            oauth: "夸克官方组件浏览器 OAuth；应用自动检测，无需配置路径".to_string(),
            upload: "官方组件上传、秒传与 NDJSON 真实进度".to_string(),
            resume: "官方组件 upload list/resume 任务恢复".to_string(),
            share: "官方组件 share，支持公开/私密链接和永久至 180 天有效期".to_string(),
            limitations: vec![
                "官方 2026-08 版本仅支持 Windows WSL，不支持原生 Windows".to_string(),
                "当前未找到面向普通桌面应用公开的 Client ID 注册与直连 REST API 文档".to_string(),
                "未安装官方组件时 Adapter 会保持不可用，不会调用私有 Cookie API".to_string(),
            ],
        },
    ]
}
