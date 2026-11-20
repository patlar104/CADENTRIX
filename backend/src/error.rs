use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use serde_json::json;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}
impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self { Self { status: StatusCode::BAD_REQUEST, code: "invalid_request", message: message.into() } }
    pub fn conflict(message: impl Into<String>) -> Self { Self { status: StatusCode::CONFLICT, code: "conflict", message: message.into() } }
    pub fn not_found() -> Self { Self { status: StatusCode::NOT_FOUND, code: "not_found", message: "Task or resource not found.".into() } }
    pub fn internal(error: impl std::fmt::Display) -> Self {
        eprintln!("cadentrix storage/runtime error: {error}");
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, code: "internal_error", message: "The local service could not complete this operation. Check the server log.".into() }
    }
    pub fn job(message: impl Into<String>) -> Self { Self { status: StatusCode::UNPROCESSABLE_ENTITY, code: "inspection_failed", message: message.into() } }
    pub fn forbidden() -> Self { Self { status: StatusCode::FORBIDDEN, code: "forbidden", message: "This request is not from an allowed local application origin.".into() } }
}
impl std::fmt::Display for AppError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}: {}", self.code, self.message) } }
impl std::error::Error for AppError {}
impl From<rusqlite::Error> for AppError { fn from(error: rusqlite::Error) -> Self { Self::internal(error) } }
impl From<serde_json::Error> for AppError { fn from(error: serde_json::Error) -> Self { Self::internal(error) } }
impl IntoResponse for AppError {
    fn into_response(self) -> Response { (self.status, Json(json!({"error": {"code": self.code, "message": self.message}}))).into_response() }
}
