use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use tracing::error;

use crate::{
    domain::wasm_module::wasm_module::WasmModuleMetadata,
    dto::responses::{
        response_data::http_resp,
        wasm_module::{GetWasmModulesResponse, WasmModuleItem},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::wasm_module,
    util::time::now::tokio_now,
};

/// GET /api/wasm-modules
/// Public endpoint - lists all WASM modules
#[utoipa::path(
    get,
    path = "/api/wasm-modules",
    tag = "wasm_module",
    responses(
        (status = 200, description = "List of WASM modules", body = GetWasmModulesResponse),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_wasm_modules(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state.get_conn().await.map_err(|e| {
        error!(error = ?e, "Failed to get DB connection");
        code_err(CodeError::POOL_ERROR, e)
    })?;

    let modules: Vec<WasmModuleMetadata> = wasm_module::table
        .select(WasmModuleMetadata::as_select())
        .order(wasm_module::wasm_module_created_at.desc())
        .load(&mut conn)
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to query WASM modules");
            code_err(CodeError::DB_QUERY_ERROR, e)
        })?;

    drop(conn);

    let items: Vec<WasmModuleItem> = modules.into_iter().map(WasmModuleItem::from).collect();

    Ok(http_resp(GetWasmModulesResponse { items }, (), start))
}
