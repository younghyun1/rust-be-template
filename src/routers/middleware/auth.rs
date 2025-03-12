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
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
};

pub async fn auth_middleware(
    State(state): State<Arc<ServerState>>,
    cookie_jar: CookieJar,
    mut request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    let session_id = match cookie_jar.get("session_id") {
        Some(session_cookie) => match Uuid::from_str(session_cookie.value()) {
            Ok(session_id) => session_id,
            Err(e) => {
                return Err(code_err(
                    CodeError::UNAUTHORIZED_ACCESS,
                    format!("Failed to parse session cookie: {}", e),
                ));
            }
        },
        None => {
            return Err(code_err(
                CodeError::UNAUTHORIZED_ACCESS,
                "Session cookie is missing".to_string(),
            ));
        }
    };

    let session = match state.get_session(&session_id).await {
        Ok(session) => session,
        Err(e) => {
            return Err(code_err(
                CodeError::UNAUTHORIZED_ACCESS,
                format!("Failed to retrieve session: {}", e),
            ));
        }
    };

    if !session.is_valid() {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "Session is invalid".to_string(),
        ));
    }

    // Assuming your Session carries a field `user_id` of type Uuid.
    request.extensions_mut().insert(session.get_user_id());

    let response = next.run(request).await;

    Ok(response)
}
