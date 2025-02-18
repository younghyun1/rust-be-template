use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use lettre::{AsyncTransport, Message};
use tracing::error;
use uuid::Uuid;

use crate::{
    domain::user::{NewEmailVerificationToken, NewUser},
    dto::{
        requests::user::signup_request::SignupRequest,
        responses::{response_data::http_resp, user::signup_response::SignupResponse},
    },
    errors::code_error::{code_err, CodeError, HandlerResponse},
    init::state::ServerState,
    schema::{email_verification_tokens, users},
    util::{
        crypto::hash_pw::hash_pw,
        string::validations::{validate_password_form, validate_username},
        time::now::tokio_now,
    },
};

const EMAIL_VERIFICATION_TOKEN_VALID_DURATION: chrono::TimeDelta = chrono::Duration::days(1);

pub async fn signup_handler(
    Extension(request_received_time): Extension<DateTime<Utc>>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SignupRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    if !validate_username(&request.user_name) {
        return Err(CodeError::USER_NAME_INVALID.into());
    }

    if !validate_password_form(&request.user_password) {
        return Err(CodeError::PASSWORD_INVALID.into());
    }

    if !email_address::EmailAddress::is_valid(&request.user_email) {
        return Err(CodeError::EMAIL_INVALID.into());
    };

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

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

    let new_user: NewUser = NewUser::new(&request.user_name, &request.user_email, &hashed_pw);

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

    let new_email_verification_token: NewEmailVerificationToken = NewEmailVerificationToken::new(
        &user_id,
        &email_verification_token,
        request_received_time + EMAIL_VERIFICATION_TOKEN_VALID_DURATION, // expires_at
        request_received_time,                                           // created_at
    );

    let inserted_email_verification_token_verify_by: DateTime<Utc> =
        diesel::insert_into(email_verification_tokens::table)
            .values(new_email_verification_token)
            .returning(email_verification_tokens::email_verification_token_expires_at)
            .get_result(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

    drop(conn);

    // TODO: Email resend handler in case this fails
    let user_email = request.user_email.clone();

    tokio::spawn(async move {
        let email_client = state.get_email_client();

        let email: Message = Message::builder()
            .from("Cyhdev Forums <donotreply@cyhdev.com>".parse().unwrap())
            .to(user_email.parse().unwrap())
            .subject("Verify your Email")
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(String::from("TEST"))
            .unwrap();

        match email_client.send(email).await {
            Ok(_) => (),
            Err(e) => {
                error!(error = %e, "Could not send email.")
            }
        };
    });

    Ok(http_resp(
        SignupResponse {
            user_name: request.user_name,
            user_email: request.user_email,
            verify_by: inserted_email_verification_token_verify_by,
        },
        (),
        start,
    ))
}
