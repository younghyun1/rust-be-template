use std::{str::FromStr, sync::Arc};

use crate::{
    DOMAIN_NAME,
    domain::user::User,
    dto::{
        requests::auth::login_request::LoginRequest,
        responses::{auth::login_response::LoginResponse, response_data::http_resp_with_cookies},
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::{DeploymentEnvironment, ServerState},
    schema::users,
    util::{
        crypto::verify_pw::verify_pw, string::validations::validate_password_form,
        time::now::tokio_now,
    },
};
use axum::{Json, extract::State, response::IntoResponse};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::{error, trace, warn};
use uuid::Uuid;
use zeroize::Zeroize;

pub async fn login(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
    Json(mut request): Json<LoginRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Check forms first to save time; this should also be done in the FE
    if !email_address::EmailAddress::is_valid(&request.user_email) {
        return Err(CodeError::EMAIL_INVALID.into());
    };

    if !validate_password_form(&request.user_password) {
        return Err(CodeError::PASSWORD_INVALID.into());
    }

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let user: User = match users::table
        .filter(users::user_email.eq(&request.user_email))
        .first::<User>(&mut conn)
        .await
    {
        Ok(user) => user,
        Err(e) => match e {
            diesel::result::Error::NotFound => {
                return Err(CodeError::USER_NOT_FOUND.into());
            }
            _ => {
                return Err(code_err(CodeError::DB_QUERY_ERROR, e));
            }
        },
    };

    match verify_pw(&request.user_password, &user.user_password_hash).await {
        Ok(true) => (),
        Ok(false) => return Err(CodeError::WRONG_PW.into()),
        Err(e) => return Err(code_err(CodeError::COULD_NOT_VERIFY_PW, e)),
    }

    // Leave no password alive in RAM!
    request.zeroize();

    // Invalidate prior session here.
    let old_session_id: Option<Uuid> = match cookie_jar.get("session_id") {
        Some(cookie) => match Uuid::from_str(cookie.value()) {
            Ok(id) => Some(id),
            Err(e) => {
                error!(session_id=%cookie.value(), error=%e, "Invalid session_id in submitted cookies.");
                None
            }
        },
        None => None,
    };

    if let Some(old_session_id) = old_session_id {
        match state.remove_session(old_session_id).await {
            Ok((removed_session_id, session_count)) => {
                trace!(removed_session_id = %removed_session_id, session_count = %session_count, "User re-logging-in; session removed.");
            }
            Err(e) => {
                warn!(error = %e, old_session_id = %old_session_id, "Could not remove session ID! Server may have been re-started.");
            }
        };
    }

    let session_id: Uuid = state
        .new_session(&user, user.user_is_email_verified, None)
        .await
        .map_err(|e| code_err(CodeError::SESSION_ID_ALREADY_EXISTS, e))?;

    // for prod
    let cookie: Cookie;

    match state.get_deployment_environment() {
        DeploymentEnvironment::Local
        | DeploymentEnvironment::Dev
        | DeploymentEnvironment::Staging => {
            cookie = Cookie::build(("session_id", session_id.to_string()))
                .path("/")
                .http_only(true)
                .domain("localhost")
                .same_site(axum_extra::extract::cookie::SameSite::Strict)
                .secure(true)
                .build();
        }
        DeploymentEnvironment::Prod => {
            cookie = Cookie::build(("session_id", session_id.to_string()))
                .path("/")
                .http_only(true)
                .domain(DOMAIN_NAME)
                .same_site(axum_extra::extract::cookie::SameSite::Strict)
                .secure(true)
                .build();
        }
    }

    drop(conn);

    Ok(http_resp_with_cookies(
        LoginResponse {
            message: "Login successful".to_string(),
            user_id: user.user_id,
        },
        (),
        start,
        Some(vec![cookie]),
        None,
    ))
}
