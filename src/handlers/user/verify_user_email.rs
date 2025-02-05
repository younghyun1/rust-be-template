use std::sync::Arc;

use crate::{
    domain::user::EmailVerificationToken,
    dto::{
        requests::user::verify_user_email_request::VerifyUserEmailRequest,
        responses::{
            response_data::http_resp, user::email_validate_response::EmailValidateResponse,
        },
    },
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    schema::{email_verification_tokens, users},
    util::time::now::tokio_now,
};

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use diesel::{dsl::exists, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

pub async fn verify_user_email(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<VerifyUserEmailRequest>,
) -> HandlerResult<impl IntoResponse> {
    let start = tokio_now();
    let now = Utc::now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

    let email_verification_token: EmailVerificationToken = email_verification_tokens::table
        .filter(
            email_verification_tokens::email_verification_token
                .eq(&request.email_verification_token),
        )
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::INVALID_EMAIL_VERIFICATION_TOKEN, e))?;

    // validate if expired
    if email_verification_token.email_verification_token_expires_at < now {
        return Err(CodeError::EMAIL_VERIFICATION_TOKEN_EXPIRED.into());
    }

    // validate if we're being messed with
    if email_verification_token.email_verification_token_created_at > now {
        return Err(CodeError::EMAIL_VERIFICATION_TOKEN_FABRICATED.into());
    }

    // validate if token was already used
    if email_verification_token
        .email_verification_token_used_at
        .is_some()
    {
        return Err(CodeError::EMAIL_VERIFICATION_TOKEN_ALREADY_USED.into());
    }

    #[rustfmt::skip]
    let user_is_email_verified: bool = diesel::select(
        exists(
            users::table.filter(users::user_id.eq(&email_verification_token.user_id)),
        ))
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    if user_is_email_verified {
        return Err(CodeError::USER_EMAIL_ALREADY_VERIFIED.into());
    }

    let updated_user_email =
        diesel::update(users::table.filter(users::user_id.eq(&email_verification_token.user_id)))
            .set((
                users::user_is_email_verified.eq(true),
                users::user_updated_at.eq(now),
            ))
            .returning(users::user_email)
            .get_result::<String>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?;

    diesel::update(email_verification_tokens::table.filter(
        email_verification_tokens::email_verification_token_id
            .eq(&email_verification_token.email_verification_token_id),
    ))
        .set(email_verification_tokens::email_verification_token_used_at.eq(now))
        .execute(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?;

    Ok(http_resp(
        EmailValidateResponse {
            user_email: updated_user_email,
            verified_at: now,
        },
        (),
        start,
    ))
}
