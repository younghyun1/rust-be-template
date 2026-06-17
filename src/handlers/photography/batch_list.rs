//! `GET /api/photographs/batches` — all of the caller's tracked batches.

use std::sync::Arc;

use axum::{Extension, extract::State, response::IntoResponse};
use uuid::Uuid;

use crate::{
    dto::responses::{
        photography::batch_status_response::BatchListResponse, response_data::http_resp,
    },
    errors::code_error::{CodeErrorResp, HandlerResponse},
    handlers::photography::batch_status::build_batch_status,
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/photographs/batches",
    tag = "photography",
    responses(
        (status = 200, description = "Caller's tracked batches", body = BatchListResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp)
    )
)]
pub async fn batch_list(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let batches = state.list_owned_batches(user_id).await;
    let mut out = Vec::with_capacity(batches.len());
    for batch in &batches {
        out.push(build_batch_status(batch).await);
    }
    // Newest batch first.
    out.sort_by_key(|batch| std::cmp::Reverse(batch.created_at));

    Ok(http_resp(BatchListResponse { batches: out }, (), start))
}
