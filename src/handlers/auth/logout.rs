use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    dto::responses::{
        auth::logout_response::LogoutResponse, response_data::http_resp_with_cookies,
    },
    errors::code_error::{CodeErrorResp, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn logout(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Construct the cookie with the same attributes as when it was set
    let mut cookie = Cookie::build(("session_id", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build();

    // Adjust the cookie to make it a removal cookie
    cookie.make_removal();

    let session_id = cookie_jar
        .get("session_id")
        .map(|cook| cook.value().to_owned());

    tokio::spawn(async move {
        if let Some(session_id) = session_id {
            match state
                .remove_session(Uuid::parse_str(&session_id).unwrap_or(Uuid::nil()))
                .await
            {
                Ok((removed_session_id, session_count)) => {
                    info!(
                        removed_session_id = %removed_session_id,
                        session_count = %session_count,
                        "User logout; session removed.",
                    );
                }
                Err(e) => {
                    error!(
                        error = %e,
                        "Failed to remove session from state",
                    );
                }
            };
        }
    });

    Ok(http_resp_with_cookies(
        LogoutResponse {
            message: "Logout successful".to_string(),
        },
        (),
        start,
        None,
        Some(vec![cookie]),
    ))
}
