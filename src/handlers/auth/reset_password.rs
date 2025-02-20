use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use chrono::Utc;
use diesel::{AsChangeset, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::error;

use crate::{
    domain::user::{PasswordResetToken, User},
    dto::{
        requests::auth::reset_password::ResetPasswordProcessRequest,
        responses::{
            auth::reset_password_response::ResetPasswordResponse, response_data::http_resp,
        },
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::users,
    util::{
        crypto::hash_pw::hash_pw, string::validations::validate_password_form, time::now::tokio_now,
    },
};

// TODO: move somewhere nice?
// is DTO for ORM -_-
#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UpdatePassword<'a> {
    user_password_hash: &'a str,
}

pub async fn reset_password(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ResetPasswordProcessRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let now = Utc::now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    if !validate_password_form(&request.new_password) {
        return Err(CodeError::PASSWORD_INVALID.into());
    }

    let password_reset_token: PasswordResetToken = crate::schema::password_reset_tokens::table
        .filter(
            crate::schema::password_reset_tokens::password_reset_token
                .eq(request.password_reset_token),
        )
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    if password_reset_token.password_reset_token_used_at.is_some() {
        return Err(CodeError::PASSWORD_RESET_TOKEN_ALREADY_USED.into());
    }

    if password_reset_token.password_reset_token_created_at > now {
        return Err(CodeError::PASSWORD_RESET_TOKEN_FABRICATED.into());
    }

    if password_reset_token.password_reset_token_expires_at < now {
        return Err(CodeError::PASSWORD_RESET_TOKEN_EXPIRED.into());
    }

    let hashed_pw = hash_pw(request.new_password)
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_HASH_PW, e))?;

    let update_data = UpdatePassword {
        user_password_hash: &hashed_pw,
    };

    // Now update the user in the users table where the id matches.
    let user: User =
        diesel::update(users::table.filter(users::user_id.eq(password_reset_token.user_id)))
            .set(&update_data)
            .returning(users::all_columns)
            .get_result(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?;

    drop(conn);

    // TODO: asynchronously flag password reset token as used
    let coroutine_state = state.clone();
    tokio::spawn(async move {
        let mut conn = match coroutine_state.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = %e, "Could not acquire connection from pool.");
                return;
            }
        };

        if let Err(e) = diesel::update(
            crate::schema::password_reset_tokens::table.filter(
                crate::schema::password_reset_tokens::password_reset_token_id
                    .eq(password_reset_token.password_reset_token_id),
            ),
        )
        .set(crate::schema::password_reset_tokens::password_reset_token_used_at.eq(now))
        .execute(&mut conn)
        .await
        {
            error!(error = %e, "Could not mark the password reset token as used.");
        }

        drop(conn);
    });

    Ok(http_resp(
        ResetPasswordResponse {
            user_id: user.user_id,
            user_name: user.user_name,
            user_email: user.user_email,
            user_updated_at: user.user_updated_at,
        },
        (),
        start,
    ))
}
