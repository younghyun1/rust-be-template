use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use diesel::prelude::Insertable;
use diesel_async::RunQueryDsl;

use crate::{
    domain::user::users,
    dto::{
        requests::user::signup_request::SignupRequest,
        responses::{response_data::http_resp, user::signup_response::SignupResponse},
    },
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::{crypto::hash_pw::hash_pw, now::t_now},
};

// TODO: where to fit this?
#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser<'a> {
    user_name: &'a str,
    user_email: &'a str,
    user_password_hash: &'a str,
}

pub async fn signup_handler<'a>(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SignupRequest>,
) -> HandlerResult<impl IntoResponse> {
    let start = t_now();

    if !validate_username(&request.user_name) {
        return Err(CodeError::USER_NAME_INVALID.into());
    }

    if !email_address::EmailAddress::is_valid(&request.user_email) {
        return Err(CodeError::EMAIL_INVALID.into());
    };

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

    let hashed_pw = hash_pw(request.user_password)
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_HASH_PW, e))?;

    let new_user = NewUser {
        user_name: &request.user_name,
        user_email: &request.user_email,
        user_password_hash: &hashed_pw,
    };

    diesel::insert_into(users::table)
        .values(new_user)
        .execute(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

    drop(conn);

    Ok(http_resp(
        SignupResponse {
            user_name: request.user_name,
            user_email: request.user_email,
        },
        (),
        start,
    ))
}

#[inline(always)]
fn validate_username<'a>(username: &'a str) -> bool {
    !username.is_empty()
}
