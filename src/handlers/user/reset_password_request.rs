use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::user::{NewPasswordResetToken, User},
    dto::{
        requests::user::reset_password_request::ResetPasswordRequest,
        responses::{
            response_data::http_resp,
            user::reset_password_request_response::ResetPasswordRequestResponse,
        },
    },
    errors::code_error::{code_err, CodeError, HandlerResponse},
    init::state::ServerState,
    schema::{password_reset_tokens, users},
    util::time::now::tokio_now,
};

const PASSWORD_RESET_TOKEN_VALID_DURATION: chrono::TimeDelta = chrono::Duration::minutes(30);

// TODO
pub async fn reset_password_request_process(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ResetPasswordRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let request_received_at = Utc::now();

    if !email_address::EmailAddress::is_valid(&request.user_email) {
        return Err(CodeError::EMAIL_INVALID.into());
    };

    let password_reset_token: Uuid = uuid::Uuid::new_v4();

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

    let new_password_reset_token: NewPasswordResetToken = NewPasswordResetToken::new(
        &user.user_id,
        &password_reset_token,
        request_received_at + PASSWORD_RESET_TOKEN_VALID_DURATION,
        request_received_at,
    );

    let inserted_password_reset_token_verify_by: DateTime<Utc> =
        diesel::insert_into(password_reset_tokens::table)
            .values(new_password_reset_token)
            .returning(password_reset_tokens::password_reset_token_expires_at)
            .get_result(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

    drop(conn);

    Ok(http_resp(
        ResetPasswordRequestResponse {
            user_email: request.user_email,
            verify_by: inserted_password_reset_token_verify_by,
        },
        (),
        start,
    ))
}
