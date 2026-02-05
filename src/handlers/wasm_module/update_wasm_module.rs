use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domain::wasm_module::wasm_module::{WasmModule, WasmModuleChangeset},
    dto::{
        requests::wasm_module::UpdateWasmModuleRequest,
        responses::{response_data::http_resp, wasm_module::WasmModuleItem},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::wasm_module,
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

/// PATCH /api/wasm-modules/{wasm_module_id}
/// Superuser only - updates WASM module metadata (title/description)
#[utoipa::path(
    patch,
    path = "/api/wasm-modules/{wasm_module_id}",
    tag = "wasm_module",
    params(
        ("wasm_module_id" = Uuid, Path, description = "WASM module UUID")
    ),
    request_body = UpdateWasmModuleRequest,
    responses(
        (status = 200, description = "WASM module updated", body = WasmModuleItem),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 404, description = "WASM module not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn update_wasm_module(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(wasm_module_id): Path<Uuid>,
    Json(body): Json<UpdateWasmModuleRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Check superuser status
    let is_su = is_superuser(state.clone(), user_id).await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to check superuser status");
        code_err(CodeError::DB_QUERY_ERROR, e)
    })?;

    if !is_su {
        error!(user_id = %user_id, "User is not superuser; cannot update WASM module");
        return Err(code_err(
            CodeError::IS_NOT_SUPERUSER,
            "Only superusers can update WASM modules",
        ));
    }

    // Build changeset
    let changeset = WasmModuleChangeset {
        wasm_module_title: body.wasm_module_title,
        wasm_module_description: body.wasm_module_description,
        wasm_module_updated_at: Some(Utc::now()),
    };

    let mut conn = state.get_conn().await.map_err(|e| {
        error!(error = ?e, "Failed to get DB connection");
        code_err(CodeError::POOL_ERROR, e)
    })?;

    let updated: WasmModule = diesel::update(
        wasm_module::table.filter(wasm_module::wasm_module_id.eq(wasm_module_id)),
    )
    .set(&changeset)
    .get_result(&mut conn)
    .await
    .map_err(|e| {
        error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to update WASM module");
        match e {
            diesel::result::Error::NotFound => {
                code_err(CodeError::DB_QUERY_ERROR, "WASM module not found")
            }
            _ => code_err(CodeError::DB_UPDATE_ERROR, e),
        }
    })?;

    drop(conn);

    info!(
        wasm_module_id = %wasm_module_id,
        user_id = %user_id,
        "WASM module updated"
    );

    Ok(http_resp(WasmModuleItem::from(updated), (), start))
}
