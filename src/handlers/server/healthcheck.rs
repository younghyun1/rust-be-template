use axum::{http::StatusCode, response::IntoResponse};

#[utoipa::path(
    get,
    path = "/api/healthcheck/server",
    responses(
        (status = 200, description = "Server is healthy")
    )
)]
pub async fn healthcheck() -> impl IntoResponse {
    (StatusCode::OK, ())
}
