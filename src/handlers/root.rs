use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use serde_derive::Serialize;

use crate::{
    dto::responses::response_data::http_resp, errors::code_error::HandlerResult,
    init::state::ServerState, util::duration_formatter::format_duration,
};

#[derive(Serialize)]
pub struct RootHandlerResponse {
    server_uptime: String,
}

pub async fn root_handler(
    State(state): State<Arc<ServerState>>,
) -> HandlerResult<impl IntoResponse> {
    let start = tokio::time::Instant::now();

    Ok(http_resp::<RootHandlerResponse, ()>(
        RootHandlerResponse {
            server_uptime: format_duration(state.get_uptime()),
        },
        (),
        start,
    ))
}
