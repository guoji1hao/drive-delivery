use crate::{
    adapters::{baidu::BaiduDriveAdapter, google::GoogleDriveAdapter, onedrive::OneDriveAdapter},
    auth::AuthManager,
    database::Database,
    errors::AppResult,
    logging::LogService,
    settings::SettingsService,
    task_manager::TaskManager,
};
use std::{fs, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub database: Arc<Database>,
    pub settings: Arc<SettingsService>,
    pub auth: Arc<AuthManager>,
    pub tasks: Arc<TaskManager>,
    pub log: Arc<LogService>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn initialize(app: AppHandle) -> AppResult<Self> {
        let _ = dotenvy::dotenv();
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| crate::errors::AppError::Other(error.to_string()))?;
        fs::create_dir_all(&data_dir)?;
        let log = Arc::new(LogService::new(&data_dir)?);
        let database = Arc::new(Database::open(&data_dir.join("dual-drive-assistant.db"))?);
        let recovered = database.recover_incomplete_tasks()?;
        if !recovered.is_empty() {
            log.info(
                "task",
                &format!("已恢复 {} 个未完成任务为暂停状态", recovered.len()),
            );
        }
        let settings = Arc::new(SettingsService::new(database.clone()));
        let auth = Arc::new(AuthManager::new(database.clone(), log.clone())?);
        let baidu = Arc::new(BaiduDriveAdapter::new(auth.clone(), log.clone())?);
        let google = Arc::new(GoogleDriveAdapter::new(auth.clone(), log.clone())?);
        let onedrive = Arc::new(OneDriveAdapter::new(auth.clone())?);
        let tasks = Arc::new(TaskManager::new(
            app,
            database.clone(),
            settings.clone(),
            baidu,
            google,
            onedrive,
            log.clone(),
        ));
        log.info("app", "双盘发货助手启动完成");
        Ok(Self {
            database,
            settings,
            auth,
            tasks,
            log,
            data_dir,
        })
    }
}
