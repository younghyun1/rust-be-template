use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use chrono::Utc;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

const UPDATE_INTERVAL: chrono::Duration = chrono::Duration::minutes(1);

#[utoipa::path(
    get,
    path = "/api/healthcheck/fastfetch",
    responses(
        (status = 200, description = "Host fastfetch information", body = String),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_host_fastfetch(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let now = Utc::now();

    if now - state.fastfetch.get_last_fetched_time().await > UPDATE_INTERVAL {
        match state.fastfetch.update_fastfetch_string().await {
            Ok(_) => (),
            Err(e) => return Err(code_err(e, "Could not update fastfetch string")),
        }
    }

    let fastfetch = state.fastfetch.get_fastfetch_string().await;

    Ok(http_resp(fastfetch, (), start))
}
