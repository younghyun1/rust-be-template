use axum::http::StatusCode;

pub struct CodeError {
    http_status_code: StatusCode,
}