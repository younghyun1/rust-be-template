use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::init::state::ServerState;

// pub async fn root_router(
//     State(state): State<Arc<ServerState>>,
// ) -> Result<impl IntoResponse, impl IntoResponse> {
// }
