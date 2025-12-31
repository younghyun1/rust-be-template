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
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::comments,
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}/{comment_id}",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post"),
        ("comment_id" = Uuid, Path, description = "ID of the comment to delete")
    ),
    responses(
        (status = 200, description = "Comment deleted successfully", body = DeleteCommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden", body = CodeErrorResp),
        (status = 404, description = "Comment not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn delete_comment(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Path((_post_id, comment_id)): Path<(Uuid, Uuid)>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // 1. Check comment author against requester ID:
    // TODO: If requester is superuser, allow deletion regardless of author

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

    let author_id: Uuid = comments::table
        .select(comments::user_id)
        .filter(comments::comment_id.eq(comment_id))
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
