use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

pub async fn get_host_fastfetch(
    State(_state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    use tokio::process::Command;

    // Run the 'fastfetch' command asynchronously
    let output = Command::new("fastfetch")
        .output()
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_RUN_FASTFETCH, e))?;

    // Convert stdout to a String, assuming UTF-8/ANSI output
    let ansi_output = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(http_resp(ansi_output, (), start))
}
