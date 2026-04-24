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
    errors::code_error::HandlerResponse,
    init::state::{ServerState, Session},
};

#[derive(Clone)]
pub enum AuthStatus {
    LoggedIn(Uuid),
    LoggedOut,
}

#[derive(Clone)]
pub struct AuthSession {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_country: i32,
}

impl From<&Session> for AuthSession {
    fn from(session: &Session) -> Self {
        Self {
            user_id: session.get_user_id(),
            user_name: session.get_user_name().to_string(),
            user_country: session.get_user_country(),
        }
    }
}

pub async fn is_logged_in_middleware(
    State(state): State<Arc<ServerState>>,
    cookie_jar: CookieJar,
    mut request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    let mut auth_session: Option<AuthSession> = None;
    let auth_status = if let Some(session_cookie) = cookie_jar.get("session_id") {
        match Uuid::from_str(session_cookie.value()) {
            Ok(session_id) => match state.get_session(&session_id).await {
                Ok(session) if session.is_unexpired() => {
                    auth_session = Some(AuthSession::from(&session));
                    AuthStatus::LoggedIn(session.get_user_id())
                }
                _ => AuthStatus::LoggedOut,
            },
            Err(_) => AuthStatus::LoggedOut,
        }
    } else {
        AuthStatus::LoggedOut
    };

    request.extensions_mut().insert(auth_status);
    request.extensions_mut().insert(auth_session);

    let response = next.run(request).await;

    Ok(response)
}
