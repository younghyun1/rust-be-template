use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    dto::requests::user::signup_request::SignupRequest,
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::now::t_now,
};

pub async fn signup<'a, 'b, 'c>(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SignupRequest<'a, 'b, 'c>>,
) -> HandlerResult<impl IntoResponse> {
    let start = t_now();

    if !email_address::EmailAddress::is_valid(&request.email) {
        return Err(code_err(CodeError::EMAIL_INVALID, "Invalid email format!"));
    };

    Ok(())
}
