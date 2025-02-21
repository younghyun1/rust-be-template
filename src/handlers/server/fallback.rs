use axum::response::IntoResponse;
use serde_derive::Serialize;

use crate::{
    dto::responses::response_data::http_resp, errors::code_error::HandlerResponse,
    util::time::now::tokio_now,
};

#[derive(Serialize)]
pub struct FallbackHandlerResponse<'a> {
    message: &'a str,
}

pub async fn fallback_handler() -> HandlerResponse<impl IntoResponse> {
    // Capture the current time as the start time
    let start = tokio_now();
    // Return an HTTP response indicating an invalid path was accessed
    Ok(http_resp::<FallbackHandlerResponse, ()>(
        FallbackHandlerResponse {
            // A message indicating that the accessed path is invalid
            message: "Invalid path! Probes, go away.",
        },
        // No additional data is returned
        (),
        // Include the start time for any relevant logging or metrics
        start,
    ))
}
