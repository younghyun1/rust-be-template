use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeErrorResp, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/visitor-board",
    tag = "server",
    responses(
        (status = 200, description = "Visitor board entries", body = [((f64, f64), u64)]),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_visitor_board_entries(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let info = state.get_visitor_board_entries().await;

    Ok(http_resp(info, (), start))
}
