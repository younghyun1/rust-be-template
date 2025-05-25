use std::{str::FromStr, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::{errors::code_error::HandlerResponse, init::state::ServerState};

#[derive(Clone)]
pub enum AuthStatus {
    LoggedIn(Uuid),
    LoggedOut,
}

pub async fn is_logged_in_middleware(
    State(state): State<Arc<ServerState>>,
    cookie_jar: CookieJar,
    mut request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    let auth_status = if let Some(session_cookie) = cookie_jar.get("session_id") {
        match Uuid::from_str(session_cookie.value()) {
            Ok(session_id) => match state.get_session(&session_id).await {
                Ok(session) if session.is_valid() => AuthStatus::LoggedIn(session.get_user_id()),
                _ => AuthStatus::LoggedOut,
            },
            Err(_) => AuthStatus::LoggedOut,
        }
    } else {
        AuthStatus::LoggedOut
    };

    request.extensions_mut().insert(auth_status);

    let response = next.run(request).await;

    Ok(response)
}
