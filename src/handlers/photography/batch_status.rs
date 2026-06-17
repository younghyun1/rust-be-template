//! `GET /api/photographs/batch/{batch_id}` — private per-user batch status.
//!
//! Returns 404 (`BATCH_NOT_FOUND`) when the batch is absent or not owned by the
//! caller; never 403 (see `server_state/photography_batches.rs` for why).

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    domain::photography::batch::session::BatchSession,
    dto::responses::{
        photography::batch_status_response::{BatchItemStatus, BatchStatusResponse},
        response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

/// Build the wire status snapshot for a batch (items sorted by creation time).
pub async fn build_batch_status(batch: &BatchSession) -> BatchStatusResponse {
    let mut items: Vec<BatchItemStatus> = batch
        .snapshot_items()
        .await
        .into_iter()
        .map(|item| BatchItemStatus {
            item_id: item.item_id,
            file_name: item.original_file_name,
            original_size_bytes: item.original_size_bytes,
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
        })
        .collect();
    items.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.item_id.cmp(&b.item_id))
    });

    BatchStatusResponse {
        batch_id: batch.batch_id,
        created_at: batch.created_at,
        total: batch.total,
        completed: batch.completed_count(),
        failed: batch.failed_count(),
        pending: batch.pending_count(),
        done: batch.is_done(),
        items,
    }
}

#[utoipa::path(
    get,
    path = "/api/photographs/batch/{batch_id}",
    tag = "photography",
    params(("batch_id" = Uuid, Path, description = "Batch session id")),
    responses(
        (status = 200, description = "Batch status", body = BatchStatusResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 404, description = "Batch not found or not owned", body = CodeErrorResp)
    )
)]
pub async fn batch_status(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(batch_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let batch = match state.get_owned_batch(batch_id, user_id).await {
        Some(batch) => batch,
        None => {
            return Err(code_err(
                CodeError::BATCH_NOT_FOUND,
                format!("batch {batch_id} not found for requester"),
            ));
        }
    };

    let resp = build_batch_status(&batch).await;
    Ok(http_resp(resp, (), start))
}
