use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use diesel::{dsl::exists, prelude::Insertable, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use crate::{
    domain::user::users,
    dto::{
        requests::user::signup_request::SignupRequest,
        responses::{response_data::http_resp, user::signup_response::SignupResponse},
    },
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::{crypto::hash_pw::hash_pw, time::now::t_now},
};

// TODO: where to fit this?
#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser<'nu> {
    user_name: &'nu str,
    user_email: &'nu str,
    user_password_hash: &'nu str,
}

pub async fn signup_handler(
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

    #[rustfmt::skip]
    let email_exists: bool = diesel::select(
        exists(
            users::table.filter(users::user_email.eq(&request.user_email)),
        ))
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    if email_exists {
        return Err(CodeError::EMAIL_MUST_BE_UNIQUE.into());
    }

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
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => code_err(CodeError::EMAIL_MUST_BE_UNIQUE, e),
            _ => code_err(CodeError::DB_INSERTION_ERROR, e),
        })?;

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
