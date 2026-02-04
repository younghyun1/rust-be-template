use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_derive::Serialize;
use utoipa::ToSchema;

use crate::build_info::{BUILD_TIME_UTC, LIB_VERSION_MAP, RUSTC_VERSION};

#[derive(Serialize, ToSchema)]
pub struct ServerHealthcheckResponse {
    pub build_time: &'static str,
    pub axum_version: String,
    pub rust_version: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/healthcheck/server",
    tag = "server",
    responses(
        (status = 200, description = "Server is healthy", body = ServerHealthcheckResponse)
    )
)]
pub async fn healthcheck() -> impl IntoResponse {
    let axum_version: Option<&crate::build_info::LibVersion> = LIB_VERSION_MAP.get("axum");
    let axum_version = match axum_version {
        Some(lib) => [lib.get_name(), lib.get_version()].concat(),
        None => String::from("Unknown"),
    };

    (
        StatusCode::OK,
        Json(ServerHealthcheckResponse {
            build_time: BUILD_TIME_UTC,
            axum_version,
            rust_version: RUSTC_VERSION,
        }),
    )
}
