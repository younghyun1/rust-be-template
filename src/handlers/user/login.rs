use std::sync::Arc;

use crate::{
    domain::user::User,
    dto::{
        requests::user::login_request::LoginRequest,
        responses::response_data::http_resp_with_cookies,
    },
    errors::code_error::{code_err, CodeError, HandlerResponse},
    init::state::ServerState,
    schema::users,
    util::{
        crypto::verify_pw::verify_pw, string::validations::validate_password_form,
        time::now::tokio_now,
    },
};
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::extract::cookie::Cookie;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

pub async fn login(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<LoginRequest>,
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

    let session_id: Uuid = state
        .new_session(user.user_id, None)
        .await
        .map_err(|e| code_err(CodeError::SESSION_ID_ALREADY_EXISTS, e))?;

    let cookie = Cookie::build(("session_id", session_id.to_string()))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build();

    drop(conn);

    Ok(http_resp_with_cookies(
        serde_json::json!({
            "message": "Login successful",
            "user_id": user.user_id
        }),
        (),
        start,
        Some(vec![cookie]),
        None,
    ))
}
