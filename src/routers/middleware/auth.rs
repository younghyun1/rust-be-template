use std::{str::FromStr, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::{
    errors::code_error::{code_err, CodeError, HandlerResponse},
    init::state::ServerState,
};

pub async fn auth_middleware(
    State(state): State<Arc<ServerState>>,
    cookie_jar: CookieJar,
    request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    let session_id = match cookie_jar.get("session_id") {
        Some(session_cookie) => match Uuid::from_str(session_cookie.value()) {
            Ok(session_id) => session_id,
            Err(e) => return Err(code_err(CodeError::UNAUTHORIZED_ACCESS, e)),
        },
        None => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
    };

    let session = match state.get_session(&session_id).await {
        Ok(session) => session,
        Err(e) => return Err(code_err(CodeError::UNAUTHORIZED_ACCESS, e)),
    };

    if !session.is_valid() {
        return Err(CodeError::UNAUTHORIZED_ACCESS.into());
    }

    let response = next.run(request).await;

    Ok(response)
}
