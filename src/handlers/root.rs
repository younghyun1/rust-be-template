use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use serde_derive::Serialize;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::{duration_formatter::format_duration, now::t_now},
};

#[derive(Serialize)]
pub struct RootHandlerResponse {
    server_uptime: String,
}

pub async fn root_handler(
    State(state): State<Arc<ServerState>>,
) -> HandlerResult<impl IntoResponse> {
    let start = t_now();

    let conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

    drop(conn);

    Ok(http_resp::<RootHandlerResponse, ()>(
        RootHandlerResponse {
            server_uptime: format_duration(state.get_uptime()),
        },
        (),
        start,
    ))
}
