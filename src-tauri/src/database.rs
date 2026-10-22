use crate::{
    errors::{AppError, AppResult},
    models::{AppSettings, HistoryRecord, PlatformState, TaskState, UploadTask},
};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS settings (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS tasks (
               id TEXT PRIMARY KEY,
               batch_id TEXT NOT NULL,
               local_path TEXT NOT NULL,
               file_name TEXT NOT NULL,
               file_size INTEGER NOT NULL,
               file_hash TEXT NOT NULL,
               state TEXT NOT NULL,
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL,
               payload TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_tasks_hash ON tasks(file_hash, file_size);
             CREATE INDEX IF NOT EXISTS idx_tasks_updated ON tasks(updated_at DESC);
             CREATE TABLE IF NOT EXISTS history (
               id TEXT PRIMARY KEY,
               file_name TEXT NOT NULL,
               file_size INTEGER NOT NULL,
               local_path TEXT NOT NULL,
               file_hash TEXT NOT NULL,
               uploaded_at INTEGER NOT NULL,
               task_status TEXT NOT NULL,
               baidu_path TEXT,
               baidu_remote_id TEXT,
               baidu_url TEXT,
               baidu_code TEXT,
               baidu_expires_at INTEGER,
               baidu_share_id TEXT,
               google_path TEXT,
               onedrive_path TEXT,
               google_remote_id TEXT,
               onedrive_remote_id TEXT,
               google_url TEXT,
               onedrive_url TEXT,
               google_code TEXT,
               onedrive_code TEXT,
               google_expires_at INTEGER,
               onedrive_expires_at INTEGER,
               google_share_id TEXT,
               onedrive_share_id TEXT,
               quark_path TEXT,
               quark_remote_id TEXT,
               quark_url TEXT,
               quark_code TEXT,
               quark_expires_at INTEGER,
               quark_share_id TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_history_name ON history(file_name);
             CREATE INDEX IF NOT EXISTS idx_history_hash ON history(file_hash, file_size);
             CREATE INDEX IF NOT EXISTS idx_history_time ON history(uploaded_at DESC);",
        )?;
        ensure_history_column(&connection, "baidu_remote_id", "TEXT")?;
        ensure_history_column(&connection, "google_path", "TEXT")?;
        ensure_history_column(&connection, "onedrive_path", "TEXT")?;
        ensure_history_column(&connection, "google_remote_id", "TEXT")?;
        ensure_history_column(&connection, "onedrive_remote_id", "TEXT")?;
        ensure_history_column(&connection, "google_url", "TEXT")?;
        ensure_history_column(&connection, "onedrive_url", "TEXT")?;
        ensure_history_column(&connection, "google_code", "TEXT")?;
        ensure_history_column(&connection, "onedrive_code", "TEXT")?;
        ensure_history_column(&connection, "google_expires_at", "INTEGER")?;
        ensure_history_column(&connection, "onedrive_expires_at", "INTEGER")?;
        ensure_history_column(&connection, "google_share_id", "TEXT")?;
        ensure_history_column(&connection, "onedrive_share_id", "TEXT")?;
        // 旧版本的夸克列保留在表结构中，避免破坏性迁移；当前版本不再读取或写入这些列。
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn save_task(&self, task: &UploadTask) -> AppResult<()> {
        let payload =
            serde_json::to_string(task).map_err(|error| AppError::Database(error.to_string()))?;
        self.connection.lock().execute(
            "INSERT INTO tasks (id,batch_id,local_path,file_name,file_size,file_hash,state,created_at,updated_at,payload)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
             ON CONFLICT(id) DO UPDATE SET
               state=excluded.state, updated_at=excluded.updated_at, payload=excluded.payload",
            params![
                task.id,
                task.batch_id,
                task.local_path,
                task.file_name,
                task.file_size as i64,
                task.file_hash,
                state_name(&task.state),
                task.created_at,
                task.updated_at,
                payload
            ],
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> AppResult<Option<UploadTask>> {
        let payload: Option<String> = self
            .connection
            .lock()
            .query_row("SELECT payload FROM tasks WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        payload.map(parse_task).transpose()
    }

    pub fn list_tasks(&self, limit: usize) -> AppResult<Vec<UploadTask>> {
        let connection = self.connection.lock();
        let mut statement =
            connection.prepare("SELECT payload FROM tasks ORDER BY updated_at DESC LIMIT ?1")?;
        let rows = statement.query_map([limit as i64], |row| row.get::<_, String>(0))?;
        rows.map(|row| parse_task(row?)).collect()
    }

    pub fn recover_incomplete_tasks(&self) -> AppResult<Vec<UploadTask>> {
        let mut tasks = self.list_tasks(500)?;
        let mut recovered = Vec::new();
        for task in &mut tasks {
            let mut changed = false;
            for platform in [&mut task.baidu, &mut task.google, &mut task.onedrive] {
                if matches!(
                    platform.state,
                    PlatformState::Waiting
                        | PlatformState::Hashing
                        | PlatformState::Uploading
                        | PlatformState::CreatingShare
                ) {
                    platform.state = PlatformState::Paused;
                    changed = true;
                }
            }
            if changed {
                task.state = if [&task.baidu, &task.google, &task.onedrive]
                    .iter()
                    .any(|p| p.state == PlatformState::Completed)
                {
                    TaskState::PartialSuccess
                } else {
                    TaskState::Paused
                };
                task.message = Some("程序上次退出时任务未完成，可按平台继续".into());
                self.save_task(task)?;
                recovered.push(task.clone());
            }
        }
        Ok(recovered)
    }

    pub fn save_history(&self, task: &UploadTask) -> AppResult<()> {
        let record = HistoryRecord::from(task);
        self.connection.lock().execute(
            "INSERT INTO history (
               id,file_name,file_size,local_path,file_hash,uploaded_at,task_status,
               baidu_path,baidu_remote_id,baidu_url,baidu_code,baidu_expires_at,baidu_share_id,
               google_path,google_remote_id,google_url,google_code,google_expires_at,google_share_id,
                    onedrive_path,onedrive_remote_id,onedrive_url,onedrive_code,onedrive_expires_at,onedrive_share_id
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)
             ON CONFLICT(id) DO UPDATE SET
               uploaded_at=excluded.uploaded_at, task_status=excluded.task_status,
               baidu_path=excluded.baidu_path, baidu_remote_id=excluded.baidu_remote_id,
               baidu_url=excluded.baidu_url,
               baidu_code=excluded.baidu_code, baidu_expires_at=excluded.baidu_expires_at,
               baidu_share_id=excluded.baidu_share_id,
               google_path=excluded.google_path, google_remote_id=excluded.google_remote_id,
               google_url=excluded.google_url, google_code=excluded.google_code,
               google_expires_at=excluded.google_expires_at,
               google_share_id=excluded.google_share_id,
               onedrive_path=excluded.onedrive_path,
               onedrive_remote_id=excluded.onedrive_remote_id,
               onedrive_url=excluded.onedrive_url,
               onedrive_code=excluded.onedrive_code,
               onedrive_expires_at=excluded.onedrive_expires_at,
               onedrive_share_id=excluded.onedrive_share_id",
            params![
                record.id,
                record.file_name,
                record.file_size as i64,
                record.local_path,
                record.file_hash,
                record.uploaded_at,
                state_name(&record.task_status),
                record.baidu_path,
                record.baidu_remote_id,
                record.baidu_url,
                record.baidu_code,
                record.baidu_expires_at,
                record.baidu_share_id,
                record.google_path,
                record.google_remote_id,
                record.google_url,
                record.google_code,
                record.google_expires_at,
                record.google_share_id,
                record.onedrive_path,
                record.onedrive_remote_id,
                record.onedrive_url,
                record.onedrive_code,
                record.onedrive_expires_at,
                record.onedrive_share_id
            ],
        )?;
        Ok(())
    }

    pub fn search_history(&self, query: &str, limit: usize) -> AppResult<Vec<HistoryRecord>> {
        let pattern = format!("%{query}%");
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT id,file_name,file_size,local_path,file_hash,uploaded_at,task_status,
                    baidu_path,baidu_remote_id,baidu_url,baidu_code,baidu_expires_at,baidu_share_id,
                    google_path,google_remote_id,google_url,google_code,google_expires_at,google_share_id,
                    onedrive_path,onedrive_remote_id,onedrive_url,onedrive_code,onedrive_expires_at,onedrive_share_id
             FROM history WHERE file_name LIKE ?1 ORDER BY uploaded_at DESC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![pattern, limit as i64], history_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn find_history_by_hash(&self, hash: &str, size: u64) -> AppResult<Option<HistoryRecord>> {
        self.connection
            .lock()
            .query_row(
                "SELECT id,file_name,file_size,local_path,file_hash,uploaded_at,task_status,
                        baidu_path,baidu_remote_id,baidu_url,baidu_code,baidu_expires_at,baidu_share_id,
                        google_path,google_remote_id,google_url,google_code,google_expires_at,google_share_id,
                    onedrive_path,onedrive_remote_id,onedrive_url,onedrive_code,onedrive_expires_at,onedrive_share_id
                 FROM history WHERE file_hash=?1 AND file_size=?2
                 ORDER BY ((baidu_path IS NOT NULL) + (google_path IS NOT NULL) + (onedrive_path IS NOT NULL)) DESC,
                          uploaded_at DESC LIMIT 1",
                params![hash, size as i64],
                history_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_settings(&self) -> AppResult<AppSettings> {
        let value: Option<String> = self
            .connection
            .lock()
            .query_row(
                "SELECT value FROM settings WHERE key='app_settings'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        match value {
            Some(value) => {
                let mut settings: AppSettings = serde_json::from_str(&value)
                    .map_err(|error| AppError::Database(error.to_string()))?;
                migrate_settings(&mut settings);
                Ok(settings)
            }
            None => Ok(AppSettings::default()),
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let value = serde_json::to_string(settings)
            .map_err(|error| AppError::Database(error.to_string()))?;
        self.connection.lock().execute(
            "INSERT INTO settings(key,value) VALUES('app_settings',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [value],
        )?;
        Ok(())
    }

    pub fn get_metadata(&self, key: &str) -> AppResult<Option<String>> {
        self.connection
            .lock()
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(Into::into)
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> AppResult<()> {
        self.connection.lock().execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

fn migrate_settings(settings: &mut AppSettings) {
    const BAIDU_ONLY_DEFAULT: &str = "文件：{filename}\n百度网盘：\n{baidu_url}\n{baidu_code_line}";
    const TWO_DRIVE_DEFAULT: &str = "文件：{filename}\n百度网盘：\n{baidu_url}\n{baidu_code_line}\nGoogle Drive：\n{google_url}\n{google_code_line}";
    if settings.copy_template == TWO_DRIVE_DEFAULT || settings.copy_template == BAIDU_ONLY_DEFAULT {
        settings.copy_template = AppSettings::default().copy_template;
    }
}

fn parse_task(payload: String) -> AppResult<UploadTask> {
    let value: serde_json::Value =
        serde_json::from_str(&payload).map_err(|error| AppError::Database(error.to_string()))?;
    let had_onedrive = value.get("onedrive").is_some();
    let had_google = value.get("google").is_some();
    let mut task: UploadTask =
        serde_json::from_value(value).map_err(|error| AppError::Database(error.to_string()))?;
    normalize_legacy_task(&mut task, had_google);
    if !had_onedrive {
        task.onedrive = crate::models::PlatformTask::waiting(task.file_size);
        task.onedrive.state = PlatformState::Paused;
        if matches!(task.state, TaskState::Completed | TaskState::Duplicate) {
            task.state = TaskState::PartialSuccess;
        }
    }
    Ok(task)
}

fn normalize_legacy_task(task: &mut UploadTask, had_google: bool) {
    let was_duplicate = task.state == TaskState::Duplicate;
    if !had_google {
        task.google = crate::models::PlatformTask::waiting(task.file_size);
        task.google.state = PlatformState::Paused;
        task.state = if task.baidu.state == PlatformState::Completed {
            TaskState::PartialSuccess
        } else {
            baidu_only_state(&task.baidu.state)
        };
        task.message = Some("旧版任务尚无 Google Drive 结果，可单独继续 Google 上传".to_string());
    }
    if was_duplicate && task.baidu.remote_path.is_none() {
        task.state = TaskState::Paused;
        task.baidu = crate::models::PlatformTask::waiting(task.file_size);
        task.baidu.state = PlatformState::Paused;
        task.google.state = PlatformState::Paused;
        task.message = Some("旧记录缺少上传结果，可按平台继续上传".to_string());
    }
}

fn baidu_only_state(state: &PlatformState) -> TaskState {
    match state {
        PlatformState::Completed => TaskState::Completed,
        PlatformState::Paused => TaskState::Paused,
        PlatformState::Cancelled | PlatformState::Skipped => TaskState::Cancelled,
        PlatformState::Waiting => TaskState::Waiting,
        PlatformState::Hashing | PlatformState::Uploading | PlatformState::CreatingShare => {
            TaskState::Processing
        }
        PlatformState::Failed | PlatformState::Retryable | PlatformState::Unavailable => {
            TaskState::Failed
        }
    }
}

fn ensure_history_column(
    connection: &Connection,
    column_name: &str,
    column_type: &str,
) -> rusqlite::Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(history)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == column_name {
            return Ok(());
        }
    }

    // Names are internal constants, never user-provided SQL fragments.
    connection.execute(
        &format!("ALTER TABLE history ADD COLUMN {column_name} {column_type}"),
        [],
    )?;
    Ok(())
}

fn state_name(state: &TaskState) -> &'static str {
    match state {
        TaskState::Waiting => "waiting",
        TaskState::Processing => "processing",
        TaskState::PartialSuccess => "partial_success",
        TaskState::Completed => "completed",
        TaskState::Paused => "paused",
        TaskState::Cancelled => "cancelled",
        TaskState::Failed => "failed",
        TaskState::Duplicate => "duplicate",
    }
}

fn parse_task_state(value: &str) -> TaskState {
    match value {
        "waiting" => TaskState::Waiting,
        "processing" => TaskState::Processing,
        "partial_success" => TaskState::PartialSuccess,
        "completed" => TaskState::Completed,
        "paused" => TaskState::Paused,
        "cancelled" => TaskState::Cancelled,
        "duplicate" => TaskState::Duplicate,
        _ => TaskState::Failed,
    }
}

fn history_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryRecord> {
    let size: i64 = row.get(2)?;
    let status: String = row.get(6)?;
    let baidu_path: Option<String> = row.get(7)?;
    let task_status = parse_task_state(&status);
    Ok(HistoryRecord {
        id: row.get(0)?,
        file_name: row.get(1)?,
        file_size: size.max(0) as u64,
        local_path: row.get(3)?,
        file_hash: row.get(4)?,
        uploaded_at: row.get(5)?,
        task_status,
        baidu_path,
        baidu_remote_id: row.get(8)?,
        baidu_url: row.get(9)?,
        baidu_code: row.get(10)?,
        baidu_expires_at: row.get(11)?,
        baidu_share_id: row.get(12)?,
        google_path: row.get(13)?,
        google_remote_id: row.get(14)?,
        google_url: row.get(15)?,
        google_code: row.get(16)?,
        google_expires_at: row.get(17)?,
        google_share_id: row.get(18)?,
        onedrive_path: row.get(19)?,
        onedrive_remote_id: row.get(20)?,
        onedrive_url: row.get(21)?,
        onedrive_code: row.get(22)?,
        onedrive_expires_at: row.get(23)?,
        onedrive_share_id: row.get(24)?,
    })
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::models::{PlatformState, PlatformTask, TaskState, UploadTask};
    use tempfile::tempdir;

    #[test]
    fn upgrades_baidu_only_default_copy_template() {
        let mut settings = crate::models::AppSettings {
            copy_template: "文件：{filename}\n百度网盘：\n{baidu_url}\n{baidu_code_line}".into(),
            ..crate::models::AppSettings::default()
        };
        super::migrate_settings(&mut settings);
        assert!(settings.copy_template.contains("{google_url}"));
    }

    #[test]
    fn task_and_history_survive_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("app.db");
        let database = Database::open(&path).unwrap();
        let mut task = UploadTask {
            destination_dirs: Default::default(),
            relative_dir: String::new(),
            is_directory: false,
            id: "task-1".into(),
            batch_id: "batch-1".into(),
            local_path: "C:/file.zip".into(),
            file_name: "file.zip".into(),
            file_size: 42,
            file_hash: "hash".into(),
            created_at: 1,
            updated_at: 2,
            state: TaskState::Completed,
            message: None,
            baidu: PlatformTask::waiting(42),
            google: PlatformTask::waiting(42),
            onedrive: PlatformTask::waiting(42),
        };
        task.relative_dir = "客户资料/子目录/".into();
        task.destination_dirs
            .insert("google".into(), "/delivery/2026-10-22/".into());
        task.onedrive.remote_id = Some("ms-item".into());
        task.onedrive.share.url = Some("https://example.test/onedrive".into());
        database.save_task(&task).unwrap();
        database.save_history(&task).unwrap();
        drop(database);

        let reopened = Database::open(&path).unwrap();
        assert_eq!(reopened.get_task("task-1").unwrap().unwrap().file_size, 42);
        let restored = reopened.get_task("task-1").unwrap().unwrap();
        assert_eq!(restored.relative_dir, "客户资料/子目录/");
        assert_eq!(restored.destination_dirs["google"], "/delivery/2026-10-22/");
        let history = reopened.search_history("file", 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].onedrive_remote_id.as_deref(), Some("ms-item"));
        assert_eq!(
            history[0].onedrive_url.as_deref(),
            Some("https://example.test/onedrive")
        );
        let mut payload = serde_json::to_value(&task).unwrap();
        payload.as_object_mut().unwrap().remove("onedrive");
        payload.as_object_mut().unwrap().remove("relativeDir");
        payload.as_object_mut().unwrap().remove("isDirectory");
        payload.as_object_mut().unwrap().remove("destinationDirs");
        let legacy = super::parse_task(payload.to_string()).unwrap();
        assert!(legacy.relative_dir.is_empty());
        assert!(legacy.destination_dirs.is_empty());
        assert!(!legacy.is_directory);
        assert_eq!(legacy.onedrive.state, PlatformState::Paused);
        assert_eq!(legacy.onedrive.total_bytes, 42);
        assert_eq!(legacy.state, TaskState::PartialSuccess);
        task.baidu.state = PlatformState::Completed;
        task.state = TaskState::PartialSuccess;
        task.onedrive.state = PlatformState::Uploading;
        reopened.save_task(&task).unwrap();
        let recovered = reopened.recover_incomplete_tasks().unwrap();
        assert_eq!(recovered[0].onedrive.state, PlatformState::Paused);
        assert_eq!(recovered[0].baidu.state, PlatformState::Completed);
    }

    #[test]
    fn upgrades_history_created_before_remote_ids() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE history (
                   id TEXT PRIMARY KEY,
                   file_name TEXT NOT NULL,
                   file_size INTEGER NOT NULL,
                   local_path TEXT NOT NULL,
                   file_hash TEXT NOT NULL,
                   uploaded_at INTEGER NOT NULL,
                   task_status TEXT NOT NULL,
                   baidu_path TEXT,
                   baidu_url TEXT,
                   baidu_code TEXT,
                   baidu_expires_at INTEGER,
                   baidu_share_id TEXT,
                   quark_path TEXT,
                   quark_url TEXT,
                   quark_code TEXT,
                   quark_expires_at INTEGER,
                   quark_share_id TEXT
                 );",
            )
            .unwrap();
        drop(connection);

        Database::open(&path).unwrap();
        let connection = rusqlite::Connection::open(path).unwrap();
        let mut statement = connection.prepare("PRAGMA table_info(history)").unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.contains(&"baidu_remote_id".to_string()));
        assert!(columns.contains(&"google_remote_id".to_string()));
        assert!(columns.contains(&"google_url".to_string()));
        assert!(columns.contains(&"onedrive_remote_id".to_string()));
        assert!(columns.contains(&"onedrive_url".to_string()));
    }

    #[test]
    fn legacy_task_payload_ignores_removed_provider_fields() {
        let mut task = UploadTask {
            destination_dirs: Default::default(),
            relative_dir: String::new(),
            is_directory: false,
            id: "task-legacy".into(),
            batch_id: "batch-legacy".into(),
            local_path: "C:/legacy.zip".into(),
            file_name: "legacy.zip".into(),
            file_size: 7,
            file_hash: "legacy-hash".into(),
            created_at: 1,
            updated_at: 2,
            state: TaskState::PartialSuccess,
            message: None,
            baidu: PlatformTask::waiting(7),
            google: PlatformTask::waiting(7),
            onedrive: PlatformTask::waiting(7),
        };
        task.baidu.state = PlatformState::Completed;
        let mut payload = serde_json::to_value(&task).unwrap();
        payload.as_object_mut().unwrap().insert(
            "quark".into(),
            serde_json::to_value(PlatformTask::waiting(7)).unwrap(),
        );
        payload.as_object_mut().unwrap().remove("google");

        let restored = super::parse_task(serde_json::to_string(&payload).unwrap()).unwrap();
        assert_eq!(restored.id, "task-legacy");
        assert_eq!(restored.baidu.total_bytes, 7);
        assert_eq!(restored.state, TaskState::PartialSuccess);
        assert_eq!(restored.google.state, PlatformState::Paused);
    }

    #[test]
    fn legacy_duplicate_without_baidu_result_becomes_resumable() {
        let task = UploadTask {
            destination_dirs: Default::default(),
            relative_dir: String::new(),
            is_directory: false,
            id: "task-quark-only".into(),
            batch_id: "batch-legacy".into(),
            local_path: "C:/legacy.zip".into(),
            file_name: "legacy.zip".into(),
            file_size: 7,
            file_hash: "legacy-hash".into(),
            created_at: 1,
            updated_at: 2,
            state: TaskState::Duplicate,
            message: None,
            baidu: PlatformTask::waiting(7),
            google: PlatformTask::waiting(7),
            onedrive: PlatformTask::waiting(7),
        };

        let mut payload = serde_json::to_value(&task).unwrap();
        payload.as_object_mut().unwrap().remove("google");
        let restored = super::parse_task(serde_json::to_string(&payload).unwrap()).unwrap();
        assert_eq!(restored.state, TaskState::Paused);
        assert_eq!(restored.baidu.state, PlatformState::Paused);
    }

    #[test]
    fn duplicate_lookup_prefers_existing_baidu_result() {
        let dir = tempdir().unwrap();
        let database = Database::open(&dir.path().join("history.db")).unwrap();
        let mut baidu = PlatformTask::waiting(7);
        baidu.state = PlatformState::Completed;
        baidu.remote_path = Some("/apps/demo/file.zip".into());
        let mut google = PlatformTask::waiting(7);
        google.state = PlatformState::Completed;
        google.remote_path = Some("My Drive/自动发货/file.zip".into());
        let with_baidu = UploadTask {
            destination_dirs: Default::default(),
            relative_dir: String::new(),
            is_directory: false,
            id: "with-baidu".into(),
            batch_id: "batch".into(),
            local_path: "C:/file.zip".into(),
            file_name: "file.zip".into(),
            file_size: 7,
            file_hash: "same-hash".into(),
            created_at: 1,
            updated_at: 1,
            state: TaskState::Completed,
            message: None,
            baidu,
            google,
            onedrive: PlatformTask::waiting(7),
        };
        let without_baidu = UploadTask {
            destination_dirs: Default::default(),
            relative_dir: String::new(),
            is_directory: false,
            id: "without-baidu".into(),
            updated_at: 2,
            baidu: PlatformTask::waiting(7),
            google: PlatformTask::waiting(7),
            onedrive: PlatformTask::waiting(7),
            ..with_baidu.clone()
        };
        database.save_history(&with_baidu).unwrap();
        database.save_history(&without_baidu).unwrap();

        let found = database
            .find_history_by_hash("same-hash", 7)
            .unwrap()
            .unwrap();
        assert_eq!(found.id, "with-baidu");
    }
}
