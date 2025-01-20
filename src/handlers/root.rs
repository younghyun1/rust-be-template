use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::{
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
};

pub async fn root_handler(
    State(state): State<Arc<ServerState>>,
) -> HandlerResult<impl IntoResponse> {
    // Use the `?` operator for error handling
    let conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

    
    
    drop(conn);

    Ok((StatusCode::OK, "dsadsa"))
}
