use std::{fs::{File, OpenOptions}, path::{Path, PathBuf}, sync::{Arc, Mutex, MutexGuard}, time::Duration};
use fs2::FileExt;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use crate::{AppError, Result, Task, TaskSummary, Run, Report, stages};
use crate::model::{id, now, ROLES};

struct Inner { connection: Mutex<Connection>, workspace: PathBuf, _lock: File }
#[derive(Clone)]
pub struct Store(Arc<Inner>);

fn task_from(connection: &Connection, id: &str) -> Result<Task> {
    let state: Option<String> = connection.query_row("SELECT state FROM tasks WHERE id = ?1", [id], |row| row.get(0)).optional()?;
    serde_json::from_str(&state.ok_or_else(AppError::not_found)?).map_err(Into::into)
}
fn save(transaction: &Transaction<'_>, task: &mut Task) -> Result<()> {
    task.revision += 1;
    transaction.execute("UPDATE tasks SET state = ?1, status = ?2 WHERE id = ?3", params![serde_json::to_string(task)?, task.status, task.id])?;
    Ok(())
}
impl Store {
    pub fn open(database: &Path, workspace: &Path) -> Result<Self> {
        let workspace = workspace.canonicalize().map_err(|_| AppError::bad_request("Configured workspace does not exist."))?;
        if !workspace.is_dir() { return Err(AppError::bad_request("Configured workspace must be a directory.")); }
        if let Some(parent) = database.parent() { std::fs::create_dir_all(parent).map_err(AppError::internal)?; }
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(database.with_extension("sqlite3.lock")).map_err(AppError::internal)?;
        lock.try_lock_exclusive().map_err(|_| AppError::conflict("Another CADENTRIX process already owns this database."))?;
        let mut connection = Connection::open(database)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 { return Err(AppError::conflict("The database schema is newer than this server supports.")); }
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch("
            CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY, state TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), status TEXT NOT NULL, started_at TEXT NOT NULL, finished_at TEXT, report_json TEXT);
            CREATE UNIQUE INDEX IF NOT EXISTS one_active_run_per_task ON runs(task_id) WHERE status = 'running';
            CREATE TABLE IF NOT EXISTS workspace_identity (singleton INTEGER PRIMARY KEY CHECK (singleton = 1), canonical_path BLOB NOT NULL);
        ")?;
        // Canonicalize before binding so dot segments and symlink aliases agree.
        // Native path bytes avoid treating different non-Unicode paths as equal.
        let supplied_identity = workspace.as_os_str().as_encoded_bytes();
        let stored_identity: Option<Vec<u8>> = transaction.query_row(
            "SELECT canonical_path FROM workspace_identity WHERE singleton = 1",
            [], |row| row.get(0),
        ).optional()?;
        match stored_identity {
            Some(identity) if identity != supplied_identity => {
                return Err(AppError::conflict("This database belongs to a different workspace."));
            }
            Some(_) => {}
            None => {
                let task_count: i64 = transaction.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
                if task_count != 0 {
                    return Err(AppError::conflict("This existing database has no workspace identity; its original scope cannot be verified."));
                }
                transaction.execute(
                    "INSERT INTO workspace_identity(singleton, canonical_path) VALUES (1, ?1)",
                    [supplied_identity],
                )?;
            }
        }
        transaction.pragma_update(None, "user_version", 1)?;
        transaction.commit()?;
        let store = Self(Arc::new(Inner { connection: Mutex::new(connection), workspace, _lock: lock }));
        store.recover_interrupted()?;
        Ok(store)
    }
    fn connection(&self) -> Result<MutexGuard<'_, Connection>> { self.0.connection.lock().map_err(|_| AppError::internal("Database mutex was poisoned")) }
    pub fn workspace(&self) -> &Path { &self.0.workspace }
    pub fn workspace_name(&self) -> String { self.workspace().file_name().unwrap_or_default().to_string_lossy().into_owned() }
    pub fn create_task(&self, name: &str, objective: &str, priority: &str) -> Result<String> {
        let name = name.trim(); let objective = objective.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) { return Err(AppError::bad_request("Task name must contain 1–80 characters without control characters.")); }
        if objective.is_empty() || objective.chars().count() > 240 || objective.chars().any(|c| c.is_control() && c != '\n' && c != '\t') { return Err(AppError::bad_request("Objective must contain 1–240 characters.")); }
        if !["Standard", "Urgent", "Background"].contains(&priority) { return Err(AppError::bad_request("Priority must be Standard, Urgent, or Background.")); }
        let timestamp = now();
        let mut task = Task { id: id(), name: name.into(), objective: objective.into(), priority: priority.into(), job_type: "project_inspection".into(), status: "ready".into(), step: -1, revision: 1, created_at: timestamp.clone(), updated_at: timestamp, run_id: None, stages: stages(), events: vec![], report: None, error: None };
        task.event("idle", "Task created", "Read-only project inspection is ready to run. Priority is recorded; starts are explicit.");
        self.connection()?.execute("INSERT INTO tasks(id, state, status, created_at) VALUES (?1, ?2, ?3, ?4)", params![task.id, serde_json::to_string(&task)?, task.status, task.created_at])?;
        Ok(task.id)
    }
    pub fn get_task(&self, id: &str) -> Result<Task> { let connection = self.connection()?; task_from(&connection, id) }
    pub fn list_tasks(&self) -> Result<Vec<TaskSummary>> {
        let connection = self.connection()?;
        let mut query = connection.prepare("SELECT state FROM tasks ORDER BY created_at DESC, rowid DESC")?;
        let rows = query.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str::<Task>(&row?)?.into())).collect()
    }
    pub fn active_runs(&self) -> Result<i64> { Ok(self.connection()?.query_row("SELECT COUNT(*) FROM runs WHERE status = 'running'", [], |row| row.get(0))?) }
    pub fn begin_run(&self, task_id: &str) -> Result<Run> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut task = task_from(&transaction, task_id)?;
        if !["ready", "failed", "interrupted"].contains(&task.status.as_str()) { return Err(AppError::conflict("This task is already running or completed. Create a new task for another inspection.")); }
        let active: i64 = transaction.query_row("SELECT COUNT(*) FROM runs WHERE status = 'running'", [], |row| row.get(0))?;
        if active >= 1 { return Err(AppError::conflict("The local worker is busy. Start this task after the current inspection finishes.")); }
        let run = Run { id: id(), task_id: task_id.into() };
        task.status = "running".into(); task.step = 0; task.error = None; task.report = None; task.run_id = Some(run.id.clone()); task.stages = stages(); task.stages[0].state = "running".into();
        task.event("running", "Planner started", "Inspection admitted to the single local worker. Validating the configured workspace.");
        transaction.execute("INSERT INTO runs(id, task_id, status, started_at) VALUES (?1, ?2, 'running', ?3)", params![run.id, task_id, task.updated_at])?;
        save(&transaction, &mut task)?;
        transaction.commit()?;
        Ok(run)
    }
    pub fn finish_stage(&self, run: &Run, index: usize, work: &str, report: Option<Report>) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut task = task_from(&transaction, &run.task_id)?;
        if task.run_id.as_deref() != Some(run.id.as_str()) || task.status != "running" || task.step != index as i32 || index > 3 { return Err(AppError::conflict("Run ownership or stage has changed; stale progress was rejected.")); }
        if index == 3 && report.is_none() { return Err(AppError::bad_request("A completed inspection requires a verified report.")); }
        task.stages[index].state = "complete".into(); task.stages[index].work = work.into();
        task.event("complete", &format!("{} completed", ROLES[index]), work);
        task.step = index as i32 + 1;
        if index < 3 {
            task.stages[index + 1].state = "running".into();
            let next_work = task.stages[index + 1].work.clone();
            task.event("running", &format!("{} started", ROLES[index + 1]), &next_work);
        } else {
            task.status = "completed".into(); task.report = report;
            transaction.execute("UPDATE runs SET status = 'completed', finished_at = ?1, report_json = ?2 WHERE id = ?3", params![task.updated_at, serde_json::to_string(&task.report)?, run.id])?;
        }
        save(&transaction, &mut task)?; transaction.commit()?; Ok(())
    }
    pub fn fail_run(&self, run: &Run, message: &str) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut task = task_from(&transaction, &run.task_id)?;
        if task.run_id.as_deref() != Some(run.id.as_str()) || task.status != "running" { return Err(AppError::conflict("Run ownership has changed; stale failure was rejected.")); }
        task.status = "failed".into(); task.error = Some(message.into());
        if let Some(stage) = task.stages.get_mut(task.step.max(0) as usize) { stage.state = "failed".into(); stage.work = message.into(); }
        task.event("failed", "Inspection failed", message);
        transaction.execute("UPDATE runs SET status = 'failed', finished_at = ?1 WHERE id = ?2", params![task.updated_at, run.id])?;
        save(&transaction, &mut task)?; transaction.commit()?; Ok(())
    }
    fn recover_interrupted(&self) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ids: Vec<String> = { let mut query = transaction.prepare("SELECT id FROM tasks WHERE status = 'running'")?; query.query_map([], |row| row.get(0))?.collect::<std::result::Result<_, _>>()? };
        for id in ids {
            let mut task = task_from(&transaction, &id)?;
            task.status = "interrupted".into(); task.error = Some("The service stopped during this run. An explicit retry is required.".into());
            for stage in &mut task.stages { if stage.state == "running" { stage.state = "waiting".into(); stage.work = "Interrupted by service restart".into(); } }
            task.event("waiting", "Run interrupted", "The service restarted before completion. No automatic replay was attempted; retry this read-only inspection explicitly.");
            save(&transaction, &mut task)?;
        }
        transaction.execute("UPDATE runs SET status = 'interrupted', finished_at = ?1 WHERE status = 'running'", [now()])?;
        transaction.commit()?; Ok(())
    }
}
