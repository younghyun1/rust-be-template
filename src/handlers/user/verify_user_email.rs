use crate::{
    dto::{
        requests::user::verify_user_email_request::VerifyUserEmailRequest,
        responses::{
            response_data::http_resp, user::email_validate_response::EmailValidateResponse,
        },
    },
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::time::now::tokio_now,
};

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::QueryableByName;
use diesel_async::RunQueryDsl;

pub async fn verify_user_email(
    State(state): State<ServerState>,
    Json(request): Json<VerifyUserEmailRequest>,
) -> HandlerResult<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

    // TODO: Make the struct and the query go somewhere else
    #[derive(QueryableByName)]
    struct UserEmail {
        #[sql_type = "diesel::sql_types::Text"]
        user_email: String,
    }

    // TODO: Make the struct and the query go somewhere else
    let query = r#"
        UPDATE users
        SET user_is_email_verified = TRUE,
            user_updated_at = NOW()
        WHERE user_is_email_verified = FALSE
          AND EXISTS (
            SELECT 1
            FROM email_verification_tokens
            WHERE email_verification_token = $1
              AND email_verification_tokens.user_id = users.user_id
          )
        RETURNING user_email;
    "#;

    let user_email: UserEmail = diesel::sql_query(query)
        .bind::<diesel::sql_types::Uuid, _>(request.email_verification_token)
        .get_result::<UserEmail>(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(database_error_kind, _) => {
                match database_error_kind {
                    diesel::result::DatabaseErrorKind::UniqueViolation
                    | diesel::result::DatabaseErrorKind::ForeignKeyViolation
                    | diesel::result::DatabaseErrorKind::NotNullViolation => {
                        code_err(CodeError::DB_UPDATE_ERROR, e)
                    }
                    _ => code_err(CodeError::DB_UPDATE_ERROR, e),
                }
            }
            diesel::result::Error::NotFound => code_err(CodeError::DB_UPDATE_ERROR, e),
            _ => code_err(CodeError::DB_UPDATE_ERROR, e),
        })?;

    let now = Utc::now();

    drop(conn);

    Ok(http_resp(
        EmailValidateResponse {
            user_email: user_email.user_email,
            verified_at: now,
        },
        (),
        start,
    ))
}
