use std::{fs, sync::{Arc, Barrier}, thread};
use cadentrix_server::{Store, execute_run, inspect_workspace};

fn fixture() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("project");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join("package.json"), "{}").unwrap();
    fs::create_dir(workspace.join("src")).unwrap();
    fs::write(workspace.join("src/main.rs"), "fn main() {}").unwrap();
    let store = Store::open(&directory.path().join("db.sqlite3"), &workspace).unwrap();
    (directory, store)
}

#[test]
fn actual_inspection_saves_report_and_complete_stage_history() {
    let (directory, store) = fixture();
    let id = store.create_task("Inspect project", "Inventory the project", "Standard").unwrap();
    let run = store.begin_run(&id).unwrap();
    execute_run(&store, &run).unwrap();
    let task = store.get_task(&id).unwrap();
    assert_eq!(task.status, "completed");
    assert_eq!(task.step, 4);
    assert!(task.stages.iter().all(|stage| stage.state == "complete"));
    let report = task.report.unwrap();
    assert_eq!(report.file_count, 2);
    assert_eq!(report.manifests, ["package.json"]);
    assert!(report.markdown.contains("src/main.rs"));
    assert!(report.markdown.contains("no AI model was invoked"));
    assert_eq!(task.events.iter().filter(|event| event.title.ends_with("completed")).count(), 4);
    assert!(task.events.iter().all(|event| chrono::DateTime::parse_from_rfc3339(&event.timestamp).is_ok()));
    assert!(store.begin_run(&id).is_err());
    assert_eq!(store.active_runs().unwrap(), 0);
    drop(store);
    let reopened = Store::open(&directory.path().join("db.sqlite3"), &directory.path().join("project")).unwrap();
    assert_eq!(reopened.get_task(&id).unwrap().report.unwrap().markdown, report.markdown);
}

#[test]
fn capacity_is_atomic_for_concurrent_starts() {
    let (_directory, store) = fixture();
    let ids: Vec<_> = (0..2).map(|i| store.create_task(&format!("Task {i}"), "Inspect", "Standard").unwrap()).collect();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = ids.into_iter().map(|id| {
        let store = store.clone(); let barrier = barrier.clone();
        thread::spawn(move || { barrier.wait(); store.begin_run(&id) })
    }).collect();
    let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(store.active_runs().unwrap(), 1);
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert!(store.begin_run(&winner.task_id).is_err());
    execute_run(&store, &winner).unwrap();
    assert_eq!(store.active_runs().unwrap(), 0);
}

#[test]
fn second_process_cannot_open_live_database() {
    let (directory, store) = fixture();
    assert!(Store::open(&directory.path().join("db.sqlite3"), store.workspace()).is_err());
}

#[test]
fn database_cannot_be_rebound_to_another_workspace() {
    let (directory, store) = fixture();
    drop(store);
    let different = directory.path().join("different-project");
    fs::create_dir(&different).unwrap();
    assert!(Store::open(&directory.path().join("db.sqlite3"), &different).is_err(), "existing task database must keep its original execution scope");
}

#[test]
fn equivalent_workspace_path_spellings_keep_the_original_binding() {
    let (directory, store) = fixture();
    let original = store.workspace().to_path_buf();
    let id = store.create_task("Keep this task", "Inspect", "Standard").unwrap();
    drop(store);
    for spelling in [original.join("."), original.join("src").join("..")] {
        let reopened = Store::open(&directory.path().join("db.sqlite3"), &spelling).unwrap();
        assert_eq!(reopened.workspace(), original);
        assert_eq!(reopened.get_task(&id).unwrap().name, "Keep this task");
    }
}

#[cfg(unix)]
#[test]
fn workspace_symlink_alias_is_accepted_but_retargeting_is_rejected() {
    let (directory, store) = fixture();
    let original = store.workspace().to_path_buf();
    let alias = directory.path().join("workspace-alias");
    std::os::unix::fs::symlink(&original, &alias).unwrap();
    drop(store);
    let reopened = Store::open(&directory.path().join("db.sqlite3"), &alias).unwrap();
    assert_eq!(reopened.workspace(), original);
    drop(reopened);
    fs::remove_file(&alias).unwrap();
    let different = directory.path().join("different-project");
    fs::create_dir(&different).unwrap();
    std::os::unix::fs::symlink(&different, &alias).unwrap();
    assert!(Store::open(&directory.path().join("db.sqlite3"), &alias).is_err());
    assert!(Store::open(&directory.path().join("db.sqlite3"), &original).is_ok());
}

#[test]
fn wrong_workspace_is_rejected_before_interrupted_run_recovery() {
    let (directory, store) = fixture();
    let original = store.workspace().to_path_buf();
    let id = store.create_task("Keep active run", "Inspect", "Standard").unwrap();
    store.begin_run(&id).unwrap();
    drop(store);
    let different = directory.path().join("different-project");
    fs::create_dir(&different).unwrap();
    let db = directory.path().join("db.sqlite3");
    assert!(Store::open(&db, &different).is_err());
    let connection = rusqlite::Connection::open(&db).unwrap();
    let status: String = connection.query_row("SELECT status FROM tasks WHERE id = ?1", [&id], |row| row.get(0)).unwrap();
    assert_eq!(status, "running", "a rejected open must not recover another workspace's run");
    drop(connection);
    let reopened = Store::open(&db, &original).unwrap();
    assert_eq!(reopened.get_task(&id).unwrap().status, "interrupted");
}

#[test]
fn existing_tasks_without_workspace_identity_cannot_be_silently_adopted() {
    let (directory, store) = fixture();
    let original = store.workspace().to_path_buf();
    let id = store.create_task("Keep original scope", "Inspect", "Standard").unwrap();
    drop(store);
    let db = directory.path().join("db.sqlite3");
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection.execute("DROP TABLE workspace_identity", []).unwrap();
    drop(connection);
    assert!(Store::open(&db, &original).is_err());
    let connection = rusqlite::Connection::open(&db).unwrap();
    let remaining: i64 = connection.query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", [&id], |row| row.get(0)).unwrap();
    assert_eq!(remaining, 1);
}

#[test]
fn interrupted_run_is_preserved_without_automatic_replay() {
    let (directory, store) = fixture();
    let id = store.create_task("Inspect project", "Inspect", "Standard").unwrap();
    let original = store.begin_run(&id).unwrap();
    store.finish_stage(&original, 0, "Scope validated", None).unwrap();
    drop(store);
    let reopened = Store::open(&directory.path().join("db.sqlite3"), &directory.path().join("project")).unwrap();
    let task = reopened.get_task(&id).unwrap();
    assert_eq!(task.status, "interrupted");
    assert_eq!(task.run_id.as_deref(), Some(original.id.as_str()));
    assert!(task.events.iter().any(|event| event.title == "Run interrupted"));
    assert_eq!(reopened.active_runs().unwrap(), 0);
    let retry = reopened.begin_run(&id).unwrap();
    assert_ne!(retry.id, original.id);
    assert!(reopened.finish_stage(&original, 1, "stale", None).is_err());
    execute_run(&reopened, &retry).unwrap();
    assert_eq!(reopened.get_task(&id).unwrap().status, "completed");
}

#[test]
fn missing_workspace_records_failed_run_without_fake_success() {
    let (_directory, store) = fixture();
    let id = store.create_task("Inspect project", "Inspect", "Standard").unwrap();
    let run = store.begin_run(&id).unwrap();
    fs::remove_dir_all(store.workspace()).unwrap();
    assert!(execute_run(&store, &run).is_err());
    let task = store.get_task(&id).unwrap();
    assert_eq!(task.status, "failed");
    assert!(task.report.is_none());
    assert!(task.error.is_some());
    assert!(!task.events.iter().any(|event| event.title == "Reviewer completed"));
    assert_eq!(store.active_runs().unwrap(), 0);
}

#[test]
fn scanner_excludes_hidden_dependencies_outputs_and_symlinks() {
    let (_directory, store) = fixture();
    for name in ["node_modules", "target", "dist", ".cadentrix", "qa"] {
        fs::create_dir(store.workspace().join(name)).unwrap();
        fs::write(store.workspace().join(name).join("excluded.txt"), "secret").unwrap();
    }
    fs::write(store.workspace().join(".env"), "SECRET=not-a-report").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/etc", store.workspace().join("outside-link")).unwrap();
    let report = inspect_workspace(store.workspace()).unwrap();
    assert_eq!(report.file_count, 2);
    assert_eq!(report.sample_files, ["package.json", "src/main.rs"]);
    assert!(!report.partial);
}

#[test]
fn depth_limit_is_reported_as_partial_instead_of_complete_inventory() {
    let (_directory, store) = fixture();
    let deep = (0..14).fold(store.workspace().to_path_buf(), |path, _| path.join("nested"));
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("too-deep.txt"), "not scanned").unwrap();
    let report = inspect_workspace(store.workspace()).unwrap();
    assert!(report.partial);
    assert!(report.notes.iter().any(|note| note.contains("depth")));
}

#[test]
fn validation_rejects_invalid_fields_and_preserves_sql_like_text() {
    let (_directory, store) = fixture();
    assert!(store.create_task("  ", "Inspect", "Standard").is_err());
    assert!(store.create_task(&"a".repeat(81), "Inspect", "Standard").is_err());
    assert!(store.create_task("Name", "  ", "Standard").is_err());
    assert!(store.create_task("Name", "Inspect", "Root").is_err());
    let name = "'; DROP TABLE tasks; --";
    let id = store.create_task(name, "Inspect", "Urgent").unwrap();
    assert_eq!(store.get_task(&id).unwrap().name, name);
    assert_eq!(store.list_tasks().unwrap().len(), 1);
}
