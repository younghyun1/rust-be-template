use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::Insertable, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::user::{email_verification_tokens, users},
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

#[derive(Insertable)]
#[diesel(table_name = email_verification_tokens)]
struct NewEmailVerificationToken<'nevt> {
    user_id: &'nevt Uuid,
    email_verification_token: &'nevt Uuid,
    email_verification_token_expires_at: DateTime<Utc>,
    email_verification_token_created_at: DateTime<Utc>,
}

pub async fn signup_handler(
    Extension(request_received_time): Extension<DateTime<Utc>>,
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

    let new_user: NewUser = NewUser {
        user_name: &request.user_name,
        user_email: &request.user_email,
        user_password_hash: &hashed_pw,
    };

    let user_id: Uuid = diesel::insert_into(users::table)
        .values(new_user)
        .returning(users::user_id)
        .get_result(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => code_err(CodeError::EMAIL_MUST_BE_UNIQUE, e),
            _ => code_err(CodeError::DB_INSERTION_ERROR, e),
        })?;

    let email_verification_token: Uuid = Uuid::new_v4();

    let new_email_verification_token: NewEmailVerificationToken = NewEmailVerificationToken {
        user_id: &user_id,
        email_verification_token: &email_verification_token,
        email_verification_token_expires_at: request_received_time + chrono::Duration::days(1),
        email_verification_token_created_at: request_received_time,
    };

    let inserted_email_verification_token: DateTime<Utc> =
        diesel::insert_into(email_verification_tokens::table)
            .values(new_email_verification_token)
            .returning(email_verification_tokens::email_verification_token_expires_at)
            .get_result(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

    // TODO: Send email with this
    // TODO: Email resend handler in case this fails

    drop(conn);

    Ok(http_resp(
        SignupResponse {
            user_name: request.user_name,
            user_email: request.user_email,
            verify_by: inserted_email_verification_token,
        },
        (),
        start,
    ))
}

#[inline(always)]
fn validate_username<'a>(username: &'a str) -> bool {
    // Enhanced validation logic for username
    let is_non_empty = !username.is_empty();
    let is_valid_length = username.len() >= 3 && username.len() <= 20;
    let is_valid_char = username.chars().all(|c| c.is_alphanumeric()); // includes hangul, etc

    is_non_empty && is_valid_length && is_valid_char
}
