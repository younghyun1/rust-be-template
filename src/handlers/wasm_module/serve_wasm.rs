use std::sync::Arc;

use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, Response, StatusCode, header},
    response::IntoResponse,
};
use tracing::{error, info};
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
    headers: HeaderMap,
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

            // Negotiate Content-Encoding: only emit gzip when the client advertises it.
            // Bundles are stored pre-compressed, so a non-gzip client must receive
            // decompressed (identity) bytes or it cannot decode the body.
            let accepts_gzip = headers
                .get(header::ACCEPT_ENCODING)
                .and_then(|v| v.to_str().ok())
                .map(|ae| {
                    ae.split(',').any(|e| {
                        let name = match e.trim().split(';').next() {
                            Some(value) => value.trim(),
                            None => "",
                        };
                        name.eq_ignore_ascii_case("gzip") || name.eq_ignore_ascii_case("x-gzip")
                    })
                })
                .unwrap_or(false);

            let serve_gzipped = is_gzipped && accepts_gzip;
            let out_bytes: Arc<[u8]> = if is_gzipped && !accepts_gzip {
                let gz = bytes.clone();
                match tokio::task::spawn_blocking(move || {
                    crate::util::wasm_bundle::gzip_decompress_limited(&gz, 256 * 1024 * 1024)
                })
                .await
                {
                    Ok(Ok(decoded)) => Arc::from(decoded.into_boxed_slice()),
                    other => {
                        error!(
                            wasm_module_id = %wasm_module_id,
                            result = ?other,
                            "Failed to decode WASM bundle for non-gzip client"
                        );
                        let mut response =
                            Response::new(Body::from("Failed to decode WASM bundle"));
                        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return response;
                    }
                }
            } else {
                bytes
            };

            let body = Body::from(Bytes::from_owner(out_bytes));

            let mut response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .header(header::VARY, header::ACCEPT_ENCODING.as_str())
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

            // Add Content-Encoding only when serving pre-compressed content to a gzip client.
            if serve_gzipped {
                response = response.header(header::CONTENT_ENCODING, "gzip");
            }

            match response.body(body) {
                Ok(response) => response,
                Err(e) => {
                    error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to build WASM response");
                    let mut response = Response::new(Body::from("Failed to build WASM response"));
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    response
                }
            }
        }
        None => match Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("WASM module not found"))
        {
            Ok(response) => response,
            Err(e) => {
                error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to build WASM not-found response");
                Response::new(Body::from("WASM module not found"))
            }
        },
    }
}
