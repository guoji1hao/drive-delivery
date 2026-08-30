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
            if matches!(task.state, TaskState::Waiting | TaskState::Processing) {
                task.state = TaskState::Paused;
                task.message = Some("程序上次退出时任务未完成，可点击继续".to_string());
                if matches!(
                    task.baidu.state,
                    PlatformState::Waiting
                        | PlatformState::Hashing
                        | PlatformState::Uploading
                        | PlatformState::CreatingShare
                ) {
                    task.baidu.state = PlatformState::Paused;
                }
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
               baidu_path,baidu_remote_id,baidu_url,baidu_code,baidu_expires_at,baidu_share_id
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET
               uploaded_at=excluded.uploaded_at, task_status=excluded.task_status,
               baidu_path=excluded.baidu_path, baidu_remote_id=excluded.baidu_remote_id,
               baidu_url=excluded.baidu_url,
               baidu_code=excluded.baidu_code, baidu_expires_at=excluded.baidu_expires_at,
               baidu_share_id=excluded.baidu_share_id",
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
                record.baidu_share_id
            ],
        )?;
        Ok(())
    }

    pub fn search_history(&self, query: &str, limit: usize) -> AppResult<Vec<HistoryRecord>> {
        let pattern = format!("%{query}%");
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT id,file_name,file_size,local_path,file_hash,uploaded_at,task_status,
                    baidu_path,baidu_remote_id,baidu_url,baidu_code,baidu_expires_at,baidu_share_id
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
                        baidu_path,baidu_remote_id,baidu_url,baidu_code,baidu_expires_at,baidu_share_id
                 FROM history WHERE file_hash=?1 AND file_size=?2
                 ORDER BY (baidu_path IS NOT NULL) DESC, uploaded_at DESC LIMIT 1",
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
                serde_json::from_str(&value).map_err(|error| AppError::Database(error.to_string()))
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

fn parse_task(payload: String) -> AppResult<UploadTask> {
    let mut task: UploadTask =
        serde_json::from_str(&payload).map_err(|error| AppError::Database(error.to_string()))?;
    normalize_legacy_task(&mut task);
    Ok(task)
}

fn normalize_legacy_task(task: &mut UploadTask) {
    if task.state == TaskState::PartialSuccess {
        task.state = baidu_only_state(&task.baidu.state);
        task.message = Some("已按百度网盘结果更新旧版任务状态".to_string());
    }
    if task.state == TaskState::Duplicate && task.baidu.remote_path.is_none() {
        task.state = TaskState::Paused;
        task.baidu = crate::models::PlatformTask::waiting(task.file_size);
        task.baidu.state = PlatformState::Paused;
        task.message = Some("旧记录中没有百度上传结果，可点击继续上传".to_string());
    }
}

fn baidu_only_state(state: &PlatformState) -> TaskState {
    match state {
        PlatformState::Completed => TaskState::Completed,
        PlatformState::Paused => TaskState::Paused,
        PlatformState::Cancelled => TaskState::Cancelled,
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
    let mut task_status = parse_task_state(&status);
    if task_status == TaskState::PartialSuccess {
        task_status = if baidu_path.is_some() {
            TaskState::Completed
        } else {
            TaskState::Failed
        };
    }
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
    })
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::models::{PlatformState, PlatformTask, TaskState, UploadTask};
    use tempfile::tempdir;

    #[test]
    fn task_and_history_survive_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("app.db");
        let database = Database::open(&path).unwrap();
        let task = UploadTask {
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
        };
        database.save_task(&task).unwrap();
        database.save_history(&task).unwrap();
        drop(database);

        let reopened = Database::open(&path).unwrap();
        assert_eq!(reopened.get_task("task-1").unwrap().unwrap().file_size, 42);
        assert_eq!(reopened.search_history("file", 10).unwrap().len(), 1);
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
    }

    #[test]
    fn legacy_task_payload_ignores_removed_provider_fields() {
        let mut task = UploadTask {
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
        };
        task.baidu.state = PlatformState::Completed;
        let mut payload = serde_json::to_value(&task).unwrap();
        payload.as_object_mut().unwrap().insert(
            "quark".into(),
            serde_json::to_value(PlatformTask::waiting(7)).unwrap(),
        );

        let restored = super::parse_task(serde_json::to_string(&payload).unwrap()).unwrap();
        assert_eq!(restored.id, "task-legacy");
        assert_eq!(restored.baidu.total_bytes, 7);
        assert_eq!(restored.state, TaskState::Completed);
    }

    #[test]
    fn legacy_duplicate_without_baidu_result_becomes_resumable() {
        let task = UploadTask {
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
        };

        let restored = super::parse_task(serde_json::to_string(&task).unwrap()).unwrap();
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
        let with_baidu = UploadTask {
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
        };
        let without_baidu = UploadTask {
            id: "without-baidu".into(),
            updated_at: 2,
            baidu: PlatformTask::waiting(7),
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
