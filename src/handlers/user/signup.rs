use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    dto::{
        requests::user::signup_request::SignupRequest,
        responses::{response_data::http_resp, user::signup_response::SignupResponse},
    },
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::now::t_now,
};

pub async fn signup_handler<'a>(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SignupRequest>,
) -> HandlerResult<impl IntoResponse> {
    let start = t_now();

    if !email_address::EmailAddress::is_valid(&request.user_email) {
        return Err(CodeError::EMAIL_INVALID.into());
    };

    let conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

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
