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
    dto::responses::{
        blog::delete_comment_response::DeleteCommentResponse, response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::comments,
    util::time::now::tokio_now,
};

pub async fn delete_comment(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Path((_post_id, comment_id)): Path<(Uuid, Uuid)>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // 1. Check comment author against requester ID:
    // TODO: If requester is superuser, allow deletion regardless of author

    let is_superuser: bool = false;

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

    // 1-2. Check who the author of the comment is
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let author_id: Uuid = comments::table
        .select(comments::user_id)
        .filter(comments::user_id.eq(comment_id))
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    if author_id == requester_id || is_superuser {
        // 2. Delete comment!
        match diesel::delete(comments::table.filter(comments::comment_id.eq(comment_id)))
            .execute(&mut conn)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    deleted_comment_id = %comment_id,
                    "Comment deleted"
                );
            }
            Err(e) => {
                return Err(code_err(CodeError::DB_DELETION_ERROR, e));
            }
        }
    } else {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "User is not authorized to delete this comment",
        ));
    }

    drop(conn);

    Ok(http_resp(
        DeleteCommentResponse {
            deleted_comment_id: comment_id,
        },
        (),
        start,
    ))
}
