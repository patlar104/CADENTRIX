use std::path::PathBuf;
use axum::{Json, Router, body::Body, extract::{DefaultBodyLimit, Path, Request, State, rejection::JsonRejection}, http::{header, StatusCode}, middleware::{self, Next}, response::{IntoResponse, Response}, routing::{any, get, post}};
use serde_json::{Value, json};
use tower_http::services::{ServeDir, ServeFile};
use crate::{AppError, CreateTask, Result, Store, Task, execute_run};
use crate::model::now;

async fn blocking<T: Send + 'static>(job: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> { tokio::task::spawn_blocking(job).await.map_err(AppError::internal)? }
fn policies(store: &Store) -> Result<Value> {
    Ok(json!({"max_concurrent_runs":1,"active_runs":store.active_runs()?,"job_type":"project_inspection","engine":"native_read_only","handoffs":"sequential","priority_scheduling":"recorded_only","cpu_telemetry":"unknown","memory_telemetry":"unknown","automatic_replay":false,"ai_models_connected":false}))
}
pub fn router(store: Store, frontend: PathBuf) -> Router {
    let index = frontend.join("index.html");
    Router::new()
        .route("/api/health", get(health))
        .route("/api/workspace", get(workspace))
        .route("/api/policies", get(policy))
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/{id}", get(get_task))
        .route("/api/tasks/{id}/runs", post(start_run))
        .route("/api/tasks/{id}/report", get(report))
        .route("/api", any(unknown_api))
        .route("/api/{*path}", any(unknown_api))
        .fallback_service(ServeDir::new(frontend).not_found_service(ServeFile::new(index)))
        .layer(DefaultBodyLimit::max(8192))
        .layer(middleware::from_fn(local_requests))
        .with_state(store)
}
async fn local_requests(request: Request, next: Next) -> Response {
    let hosts = ["127.0.0.1:4180", "localhost:4180", "127.0.0.1:4173", "localhost:4173"];
    let origins = ["http://127.0.0.1:4173", "http://localhost:4173", "http://127.0.0.1:4180", "http://localhost:4180"];
    if !request.headers().get(header::HOST).and_then(|value| value.to_str().ok()).is_some_and(|host| hosts.contains(&host)) { return AppError::forbidden().into_response(); }
    if request.headers().get(header::ORIGIN).is_some_and(|origin| !origin.to_str().ok().is_some_and(|origin| origins.contains(&origin))) { return AppError::forbidden().into_response(); }
    if request.method() != "GET" && request.method() != "HEAD" {
        if request.headers().get("sec-fetch-site").is_some_and(|value| value == "cross-site") { return AppError::forbidden().into_response(); }
        if !request.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).is_some_and(|value| value.split(';').next().is_some_and(|value| value.trim() == "application/json")) { return AppError::bad_request("Mutation requests require application/json.").into_response(); }
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response.headers_mut().insert("x-content-type-options", header::HeaderValue::from_static("nosniff"));
    response
}
async fn unknown_api() -> AppError { AppError::not_found() }
async fn health() -> Json<Value> { Json(json!({"status":"ok","server_time":now(),"version":env!("CARGO_PKG_VERSION")})) }
async fn workspace(State(store): State<Store>) -> Result<Json<Value>> {
    blocking(move || Ok(Json(json!({"workspace_name":store.workspace_name(),"server_time":now(),"policy":policies(&store)?,"tasks":store.list_tasks()?})))).await
}
async fn policy(State(store): State<Store>) -> Result<Json<Value>> { blocking(move || Ok(Json(policies(&store)?))).await }
async fn list_tasks(State(store): State<Store>) -> Result<Json<Value>> { blocking(move || Ok(Json(json!({"tasks":store.list_tasks()?})))).await }
async fn get_task(State(store): State<Store>, Path(id): Path<String>) -> Result<Json<Task>> { blocking(move || Ok(Json(store.get_task(&id)?))).await }
async fn create_task(State(store): State<Store>, payload: std::result::Result<Json<CreateTask>, JsonRejection>) -> Result<impl IntoResponse> {
    let Json(payload) = payload.map_err(|_| AppError::bad_request("Supply JSON with name, objective and priority only."))?;
    blocking(move || { let id = store.create_task(&payload.name, &payload.objective, &payload.priority)?; Ok((StatusCode::CREATED, Json(store.get_task(&id)?))) }).await
}
async fn start_run(State(store): State<Store>, Path(id): Path<String>) -> Result<impl IntoResponse> {
    let admission_store = store.clone();
    let (run, task) = blocking(move || { let run = admission_store.begin_run(&id)?; let task = admission_store.get_task(&id)?; Ok((run, task)) }).await?;
    tokio::task::spawn_blocking(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute_run(&store, &run)));
        match result {
            Ok(Ok(())) => {},
            Ok(Err(error)) => eprintln!("inspection failed: {error}"),
            Err(_) => { if let Err(error) = store.fail_run(&run, "The native worker stopped unexpectedly. An explicit retry is required.") { eprintln!("worker failure could not be recorded: {error}"); } }
        }
    });
    Ok((StatusCode::ACCEPTED, Json(task)))
}
async fn report(State(store): State<Store>, Path(id): Path<String>) -> Result<Response<Body>> {
    blocking(move || {
        let task = store.get_task(&id)?;
        let report = task.report.ok_or_else(|| AppError::conflict("A saved report is not available for this task yet."))?;
        let mut response = Response::new(Body::from(report.markdown));
        response.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("text/markdown; charset=utf-8"));
        response.headers_mut().insert(header::CONTENT_DISPOSITION, header::HeaderValue::from_static("attachment; filename=project-inspection.md"));
        Ok(response)
    }).await
}
