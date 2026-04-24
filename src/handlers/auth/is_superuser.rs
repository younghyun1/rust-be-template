use axum::{Extension, response::IntoResponse};

use crate::{
    dto::responses::{auth::is_superuser_response::IsSuperuserResponse, response_data::http_resp},
    errors::code_error::{CodeErrorResp, HandlerResponse},
    routers::middleware::is_logged_in::AuthSession,
    util::time::now::tokio_now,
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
    Extension(auth_session): Extension<Option<AuthSession>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let allowed = match auth_session {
        Some(auth_session) => auth_session.role_type.is_superuser(),
        None => false,
    };

    Ok(http_resp(
        IsSuperuserResponse {
            is_superuser: allowed,
        },
        (),
        start,
    ))
}
