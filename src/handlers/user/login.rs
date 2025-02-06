use axum::{extract::State, response::IntoResponse};

use crate::{errors::code_error::HandlerResponse, init::state::ServerState};

pub async fn login(State(state): State<ServerState>) -> HandlerResponse<impl IntoResponse> {
    Ok(())
}
