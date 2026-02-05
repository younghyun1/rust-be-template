use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde_derive::Serialize;
use tracing::{error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::wasm_module,
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteWasmModuleResponse {
    pub deleted_wasm_module_id: Uuid,
}

/// DELETE /api/wasm-modules/{wasm_module_id}
/// Superuser only - deletes a WASM module (DB record, filesystem, and cache)
#[utoipa::path(
    delete,
    path = "/api/wasm-modules/{wasm_module_id}",
    tag = "wasm_module",
    params(
        ("wasm_module_id" = Uuid, Path, description = "WASM module UUID")
    ),
    responses(
        (status = 200, description = "WASM module deleted", body = DeleteWasmModuleResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 404, description = "WASM module not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn delete_wasm_module(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(wasm_module_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Check superuser status
    let is_su = is_superuser(state.clone(), user_id).await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to check superuser status");
        code_err(CodeError::DB_QUERY_ERROR, e)
    })?;

    if !is_su {
        error!(user_id = %user_id, "User is not superuser; cannot delete WASM module");
        return Err(code_err(
            CodeError::IS_NOT_SUPERUSER,
            "Only superusers can delete WASM modules",
        ));
    }

    let mut conn = state.get_conn().await.map_err(|e| {
        error!(error = ?e, "Failed to get DB connection");
        code_err(CodeError::POOL_ERROR, e)
    })?;

    // Delete from database
    let deleted_count = diesel::delete(
        wasm_module::table.filter(wasm_module::wasm_module_id.eq(wasm_module_id)),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| {
        error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to delete WASM module from DB");
        code_err(CodeError::DB_DELETION_ERROR, e)
    })?;

    drop(conn);

    if deleted_count == 0 {
        return Err(code_err(CodeError::DB_QUERY_ERROR, "WASM module not found"));
    }

    // Remove from cache
    state.invalidate_wasm_module(wasm_module_id).await;

    // Delete from filesystem
    let candidate_paths = [
        format!("./wasm/{}.html.gz", wasm_module_id),
        format!("./wasm/{}.html", wasm_module_id),
        format!("./wasm/{}.wasm.gz", wasm_module_id),
        format!("./wasm/{}.wasm", wasm_module_id),
    ];

    for path in candidate_paths {
        if let Err(e) = tokio::fs::remove_file(&path).await {
            // Log but don't fail - file might already be gone
            warn!(
                error = ?e,
                path = %path,
                "Failed to delete WASM bundle file from filesystem (may already be deleted)"
            );
        }
    }

    info!(
        wasm_module_id = %wasm_module_id,
        user_id = %user_id,
        "WASM module deleted"
    );

    Ok(http_resp(
        DeleteWasmModuleResponse {
            deleted_wasm_module_id: wasm_module_id,
        },
        (),
        start,
    ))
}
