use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    dto::requests::user::reset_password_request::ResetPasswordRequest,
    errors::code_error::{CodeError, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

pub async fn reset_password_request_process(
    State(state): State<ServerState>,
    Json(request): Json<ResetPasswordRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    if !email_address::EmailAddress::is_valid(&request.user_email) {
        return Err(CodeError::EMAIL_INVALID.into());
    };

    Ok(())
}
