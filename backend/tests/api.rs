use axum::{Router, body::{Body, to_bytes}, http::{Request, StatusCode}};
use cadentrix_server::{Store, router};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("project");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(workspace.join("README.md"), "Example").unwrap();
    let store = Store::open(&directory.path().join("state.sqlite3"), &workspace).unwrap();
    let application = router(store, workspace.join("dist"));
    (directory, application)
}
async fn request(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app.clone().oneshot(Request::builder().method(method).uri(path).header("host", "127.0.0.1:4180").header("content-type", "application/json").body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 2_000_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn create_run_and_download_real_report_over_http_routes() {
    let (_directory, app) = app();
    let (status, workspace) = request(&app, "GET", "/api/workspace", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert!(workspace["tasks"].as_array().unwrap().is_empty());
    let (status, task) = request(&app, "POST", "/api/tasks", json!({"name":"Inspect project","objective":"Inventory project","priority":"Standard"})).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = task["id"].as_str().unwrap();
    assert_eq!(request(&app, "GET", &format!("/api/tasks/{id}/report"), Value::Null).await.0, StatusCode::CONFLICT);
    assert_eq!(request(&app, "POST", &format!("/api/tasks/{id}/runs"), json!({})).await.0, StatusCode::ACCEPTED);
    let mut completed = false;
    for _ in 0..100 {
        let (_, task) = request(&app, "GET", &format!("/api/tasks/{id}"), Value::Null).await;
        if task["status"] == "completed" { assert_eq!(task["report"]["file_count"], 1); completed = true; break; }
        tokio::task::yield_now().await;
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(completed, "real worker should publish a report");
    let response = app.oneshot(Request::builder().uri(format!("/api/tasks/{id}/report")).header("host", "127.0.0.1:4180").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-disposition"], "attachment; filename=project-inspection.md");
    assert!(String::from_utf8(to_bytes(response.into_body(), 2_000_000).await.unwrap().to_vec()).unwrap().contains("README.md"));
}

#[tokio::test]
async fn rejects_unknown_execution_parameters_and_bad_names() {
    let (_directory, app) = app();
    for body in [json!({"name":"Task","objective":"Inspect","priority":"Standard","workspace":"/etc"}), json!({"name":" ","objective":"Inspect","priority":"Standard"}), json!({"name":"Task","objective":"Inspect","priority":"Standard","command":"rm -rf /"})] {
        let (status, error) = request(&app, "POST", "/api/tasks", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["error"]["code"], "invalid_request");
    }
    assert_eq!(request(&app, "GET", "/api/tasks/missing", Value::Null).await.0, StatusCode::NOT_FOUND);
    assert_eq!(request(&app, "GET", "/api/not-real", Value::Null).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rejects_foreign_origins_and_rebinding_hosts() {
    let (_directory, app) = app();
    for (host, origin) in [("127.0.0.1:4180", "https://untrusted.example"), ("untrusted.example:4180", "http://127.0.0.1:4173")] {
        let response = app.clone().oneshot(Request::builder().method("POST").uri("/api/tasks").header("host", host).header("origin", origin).header("content-type", "application/json").body(Body::from("{}")).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let response = app.oneshot(Request::builder().method("POST").uri("/api/tasks").header("host", "127.0.0.1:4180").header("content-type", "text/plain").body(Body::from("{}")).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
