use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::auth::is_superuser::is_superuser,
};

/// Middleware that allows the request to proceed only if the requester is a superuser.
///
/// Prerequisite:
/// - `auth_middleware` must have run earlier and inserted the authenticated `Uuid` user id into
///   request extensions (see `request.extensions()`).
///
/// If the user is not authenticated or not a superuser, this returns `UNAUTHORIZED_ACCESS`.
pub async fn is_superuser_middleware(
    State(state): State<Arc<ServerState>>,
    request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    let user_id =
        request.extensions().get::<Uuid>().copied().ok_or_else(|| {
            code_err(CodeError::UNAUTHORIZED_ACCESS, "Missing user id in request")
        })?;

    let allowed = is_superuser(state, user_id)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e.to_string()))?;

    if !allowed {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "Superuser access required",
        ));
    }

    Ok(next.run(request).await)
}
