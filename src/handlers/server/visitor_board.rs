use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::response_data::http_resp, errors::code_error::HandlerResponse,
    init::state::ServerState, util::time::now::tokio_now,
};

pub async fn get_visitor_board_entries(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let info = state.get_visitor_board_entries().await;

    Ok(http_resp(info, (), start))
}
