use crate::models::{AppError, AppSettings, ServiceProfile, StoredResult, Task, TaskStatus};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("at least one service profile is required")]
    LastService,
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Clone)]
pub struct Store {
    connection: Arc<Mutex<Connection>>,
    data_dir: PathBuf,
}

impl Store {
    pub fn new(data_dir: PathBuf, default_output_root: String) -> StoreResult<Self> {
        fs::create_dir_all(&data_dir)?;
        let connection = Connection::open(data_dir.join("somarkclient.db"))?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS services (
               id TEXT PRIMARY KEY,
               data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS tasks (
               id TEXT PRIMARY KEY,
               batch_id TEXT NOT NULL,
               status TEXT NOT NULL,
               priority INTEGER NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               data TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
             CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at DESC);",
        )?;
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
            data_dir,
        };
        if store.list_services()?.is_empty() {
            store.save_service(&ServiceProfile::official())?;
        }
        if store.get_settings_optional()?.is_none() {
            let mut settings = AppSettings::new(default_output_root);
            settings.active_service_id = store.list_services()?.first().map(|item| item.id.clone());
            store.save_settings(&settings)?;
        }
        store.recover_tasks()?;
        Ok(store)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn list_services(&self) -> StoreResult<Vec<ServiceProfile>> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare("SELECT data FROM services ORDER BY rowid")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut services = Vec::new();
        for row in rows {
            services.push(serde_json::from_str(&row?)?);
        }
        Ok(services)
    }

    pub fn get_service(&self, id: &str) -> StoreResult<ServiceProfile> {
        let connection = self.connection.lock();
        let data = connection
            .query_row("SELECT data FROM services WHERE id = ?1", [id], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
            .ok_or_else(|| StoreError::NotFound(id.into()))?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save_service(&self, service: &ServiceProfile) -> StoreResult<()> {
        self.connection.lock().execute(
            "INSERT INTO services(id, data) VALUES(?1, ?2)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![service.id, serde_json::to_string(service)?],
        )?;
        Ok(())
    }

    pub fn delete_service(&self, id: &str) -> StoreResult<()> {
        if self.list_services()?.len() <= 1 {
            return Err(StoreError::LastService);
        }
        self.connection
            .lock()
            .execute("DELETE FROM services WHERE id = ?1", [id])?;
        let mut settings = self.settings()?;
        if settings.active_service_id.as_deref() == Some(id) {
            settings.active_service_id = self.list_services()?.first().map(|item| item.id.clone());
            self.save_settings(&settings)?;
        }
        Ok(())
    }

    pub fn settings(&self) -> StoreResult<AppSettings> {
        self.get_settings_optional()?
            .ok_or_else(|| StoreError::NotFound("settings".into()))
    }

    fn get_settings_optional(&self) -> StoreResult<Option<AppSettings>> {
        let connection = self.connection.lock();
        let data = connection
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;
        data.map(|value| serde_json::from_str(&value).map_err(StoreError::from))
            .transpose()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> StoreResult<()> {
        self.connection.lock().execute(
            "INSERT INTO settings(key, value) VALUES('app', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    pub fn create_tasks(&self, tasks: &[Task]) -> StoreResult<()> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        for task in tasks {
            transaction.execute(
                "INSERT INTO tasks(id, batch_id, status, priority, created_at, updated_at, data)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    task.id,
                    task.batch_id,
                    task.status.as_str(),
                    task.priority,
                    task.created_at,
                    task.updated_at,
                    serde_json::to_string(task)?
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn save_task(&self, task: &Task) -> StoreResult<()> {
        self.connection.lock().execute(
            "UPDATE tasks SET status = ?2, priority = ?3, updated_at = ?4, data = ?5 WHERE id = ?1",
            params![
                task.id,
                task.status.as_str(),
                task.priority,
                task.updated_at,
                serde_json::to_string(task)?
            ],
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> StoreResult<Task> {
        let connection = self.connection.lock();
        let data = connection
            .query_row("SELECT data FROM tasks WHERE id = ?1", [id], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
            .ok_or_else(|| StoreError::NotFound(id.into()))?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn list_tasks(&self) -> StoreResult<Vec<Task>> {
        let connection = self.connection.lock();
        let mut statement =
            connection.prepare("SELECT data FROM tasks ORDER BY created_at DESC, rowid DESC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(serde_json::from_str(&row?)?);
        }
        Ok(tasks)
    }

    pub fn runnable_tasks(&self) -> StoreResult<Vec<Task>> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT data FROM tasks
             WHERE status IN ('queued', 'waiting', 'downloading')
             ORDER BY priority DESC, created_at ASC, rowid ASC",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(serde_json::from_str(&row?)?);
        }
        Ok(tasks)
    }

    pub fn delete_task(&self, id: &str) -> StoreResult<()> {
        self.connection
            .lock()
            .execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        let result_dir = self.data_dir.join("results").join(id);
        if result_dir.exists() {
            fs::remove_dir_all(result_dir)?;
        }
        Ok(())
    }

    pub fn clear_history(&self, older_than_days: Option<u32>) -> StoreResult<usize> {
        let cutoff = older_than_days
            .map(|days| (chrono::Utc::now() - chrono::Duration::days(days.into())).to_rfc3339());
        let tasks = self.list_tasks()?;
        let mut removed = 0;
        for task in tasks {
            if !task.status.is_terminal() {
                continue;
            }
            if cutoff
                .as_ref()
                .is_some_and(|value| task.updated_at >= *value)
            {
                continue;
            }
            self.delete_task(&task.id)?;
            removed += 1;
        }
        Ok(removed)
    }

    pub fn next_priority(&self) -> StoreResult<i64> {
        let connection = self.connection.lock();
        Ok(connection.query_row(
            "SELECT COALESCE(MAX(priority), 0) + 1 FROM tasks",
            [],
            |row| row.get(0),
        )?)
    }

    pub fn reorder_task(&self, id: &str, priority: i64) -> StoreResult<()> {
        let mut task = self.get_task(id)?;
        task.priority = priority;
        self.save_task(&task)
    }

    pub fn save_result(&self, task_id: &str, result: &StoredResult) -> StoreResult<String> {
        let directory = self.data_dir.join("results").join(task_id);
        let path = directory.join("result.json");
        atomic_write(&path, serde_json::to_vec_pretty(result)?.as_slice())?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn load_result(&self, path: &str) -> StoreResult<StoredResult> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    fn recover_tasks(&self) -> StoreResult<()> {
        for mut task in self.list_tasks()? {
            if task.status == TaskStatus::Uploading {
                task.status = TaskStatus::SubmissionUnknown;
                task.error = Some(AppError::new(
                    "submissionUnknown",
                    "The previous upload ended before the server response was confirmed. Review before retrying.",
                ));
                self.save_task(&task)?;
            }
        }
        Ok(())
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!(
        "{}.part",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
    ));
    fs::write(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)
}
