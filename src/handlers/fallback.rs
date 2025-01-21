use axum::response::IntoResponse;
use serde_derive::Serialize;

use crate::{
    dto::responses::response_data::http_resp, errors::code_error::HandlerResult, util::now::t_now,
};

#[derive(Serialize)]
pub struct FallbackHandlerResponse<'a> {
    message: &'a str,
}

pub async fn fallback_handler() -> HandlerResult<impl IntoResponse> {
    let start = t_now();
    Ok(http_resp::<FallbackHandlerResponse, ()>(
        FallbackHandlerResponse {
            message: "Invalid path!",
        },
        (),
        start,
    ))
}
