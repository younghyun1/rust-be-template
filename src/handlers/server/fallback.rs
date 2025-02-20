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
    let start = tokio_now();
    Ok(http_resp::<FallbackHandlerResponse, ()>(
        FallbackHandlerResponse {
            message: "Invalid path! Probes, go away.",
        },
        (),
        start,
    ))
}
