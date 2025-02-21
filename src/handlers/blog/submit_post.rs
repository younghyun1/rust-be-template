use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{errors::code_error::HandlerResponse, init::state::ServerState};

pub async fn submit_post(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    Ok(())
}
