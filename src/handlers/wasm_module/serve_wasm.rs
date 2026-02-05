use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use tracing::info;
use uuid::Uuid;

use crate::init::state::ServerState;

/// GET /api/wasm-modules/{wasm_module_id}/wasm
/// Public endpoint - serves the WASM bundle from the in-memory cache (DB-backed)
/// Bundles are stored and served as pre-compressed .gz for smaller transfer size
#[utoipa::path(
    get,
    path = "/api/wasm-modules/{wasm_module_id}/wasm",
    tag = "wasm_module",
    params(
        ("wasm_module_id" = Uuid, Path, description = "WASM module UUID")
    ),
    responses(
        (status = 200, description = "WASM bundle", content_type = "application/wasm"),
        (status = 404, description = "WASM module not found")
    )
)]
pub async fn serve_wasm(
    State(state): State<Arc<ServerState>>,
    Path(wasm_module_id): Path<Uuid>,
) -> impl IntoResponse {
    // Get from cache or load from filesystem
    match state.get_wasm_module(wasm_module_id).await {
        Some((bytes, is_gzipped, content_type)) => {
            info!(
                wasm_module_id = %wasm_module_id,
                size_bytes = bytes.len(),
                is_gzipped = is_gzipped,
                content_type = content_type,
                "Serving WASM module bundle"
            );

            // Clone the bytes from Arc for the body
            let body = Body::from((*bytes).clone());

            let mut response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

            // Add Content-Encoding if serving pre-compressed content
            if is_gzipped {
                response = response.header(header::CONTENT_ENCODING, "gzip");
            }

            response.body(body).unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("WASM module not found"))
            .unwrap(),
    }
}
