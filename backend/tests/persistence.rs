use cadentrix_server::Store;

#[test]
fn task_survives_closing_and_reopening_database() {
    let fixture = tempfile::tempdir().unwrap();
    let db = fixture.path().join("state.sqlite3");
    let id = {
        let store = Store::open(&db, fixture.path()).unwrap();
        store.create_task("Inspect project", "Inventory this workspace", "Standard")
            .expect("creating a task should persist it")
    };
    let reopened = Store::open(&db, fixture.path()).unwrap();
    assert_eq!(reopened.get_task(&id).unwrap().name, "Inspect project");
}
