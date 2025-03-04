use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    errors::code_error::{CodeError, HandlerResponse},
    init::state::ServerState,
};

pub async fn api_key_check_middleware(
    State(state): State<Arc<ServerState>>,
    request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    let headers = request.headers();
    let api_key: Uuid = match headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .and_then(|key_str| uuid::Uuid::parse_str(key_str).ok())
    {
        Some(id) => id,
        None => {
            return Err(CodeError::API_KEY_INVALID.into());
        }
    };

    if !state.check_api_key(&api_key).await {
        return Err(CodeError::API_KEY_INVALID.into());
    }

    let response = next.run(request).await;
    Ok(response)
}
