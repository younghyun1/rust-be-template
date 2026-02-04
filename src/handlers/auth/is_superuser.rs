use std::sync::Arc;

use axum::{Extension, extract::State, response::IntoResponse};

use crate::{
    dto::responses::{auth::is_superuser_response::IsSuperuserResponse, response_data::http_resp},
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

#[utoipa::path(
    get,
    path = "/api/auth/is-superuser",
    tag = "auth",
    responses(
        (status = 200, description = "Superuser status for current user", body = IsSuperuserResponse),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn is_superuser_handler(
    Extension(auth_status): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let allowed = match auth_status {
        AuthStatus::LoggedIn(user_id) => is_superuser(state, user_id)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?,
        AuthStatus::LoggedOut => false,
    };

    Ok(http_resp(
        IsSuperuserResponse {
            is_superuser: allowed,
        },
        (),
        start,
    ))
}
