use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    dto::responses::{blog::delete_post_response::DeletePostResponse, response_data::http_resp},
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::posts,
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

pub async fn delete_post(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // 1. Check post author against requester ID:
    // 1-1. Extract requester ID from extension
    let requester_id: Uuid = match is_logged_in {
        AuthStatus::LoggedIn(id) => id,
        AuthStatus::LoggedOut => {
            return Err(code_err(
                CodeError::UNAUTHORIZED_ACCESS,
                "Unauthorized deletion request!",
            ));
        }
    };

    let is_superuser: bool = match is_superuser(state.clone(), requester_id).await {
        Ok(is_superuser) => is_superuser,
        Err(e) => return Err(code_err(CodeError::DB_QUERY_ERROR, e)),
    };

    // 1-2. Check who the author of the comment is
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let author_id: Uuid = posts::table
        .select(posts::user_id)
        .filter(posts::user_id.eq(post_id))
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    if author_id == requester_id || is_superuser {
        // 2. Delete comment!
        match diesel::delete(posts::table.filter(posts::post_id.eq(post_id)))
            .execute(&mut conn)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    deleted_post_id = %post_id,
                    "Post deleted"
                );
            }
            Err(e) => {
                return Err(code_err(CodeError::DB_DELETION_ERROR, e));
            }
        }
    } else {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "User is not authorized to delete this post",
        ));
    }

    drop(conn);

    Ok(http_resp(
        DeletePostResponse {
            deleted_post_id: post_id,
        },
        (),
        start,
    ))
}
