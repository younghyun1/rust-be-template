use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::{
        admin::sync_i18n_cache_response::SyncI18nCacheResponse, response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

pub async fn sync_i18n_cache(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let num_rows: usize = state
        .sync_i18n_data()
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_SYNC_18N_CACHE, e))?;

    Ok(http_resp(SyncI18nCacheResponse { num_rows }, (), start))
}
