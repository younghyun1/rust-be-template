//! `DELETE /api/photographs/{photograph_id}/{comment_id}` — hard-delete a
//! comment. Author or superuser only. FK `ON DELETE CASCADE` removes child
//! comments and the comment's votes.

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::auth::role::RoleType,
    dto::responses::{
        photography::delete_photograph_comment_response::DeletePhotographCommentResponse,
        response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::photograph_comments,
    util::time::now::tokio_now,
};

#[utoipa::path(
    delete,
    path = "/api/photographs/{photograph_id}/{comment_id}",
    tag = "photography",
    params(
        ("photograph_id" = Uuid, Path, description = "Photograph id"),
        ("comment_id" = Uuid, Path, description = "Comment to delete")
    ),
    responses(
        (status = 200, description = "Comment deleted", body = DeletePhotographCommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 404, description = "Comment not found", body = CodeErrorResp)
    )
)]
pub async fn delete_photograph_comment(
    Extension(requester_id): Extension<Uuid>,
    Extension(role_type): Extension<RoleType>,
    State(state): State<Arc<ServerState>>,
    Path((_photograph_id, comment_id)): Path<(Uuid, Uuid)>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let is_superuser = role_type.is_superuser();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let author_id: Uuid = photograph_comments::table
        .select(photograph_comments::user_id)
        .filter(photograph_comments::photograph_comment_id.eq(comment_id))
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
        .ok_or_else(|| code_err(CodeError::COMMENT_NOT_FOUND, "Comment not found"))?;

    if author_id != requester_id && !is_superuser {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "User is not authorized to delete this comment",
        ));
    }

    diesel::delete(
        photograph_comments::table
            .filter(photograph_comments::photograph_comment_id.eq(comment_id)),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| code_err(CodeError::DB_DELETION_ERROR, e))?;

    drop(conn);

    Ok(http_resp(
        DeletePhotographCommentResponse {
            deleted_photograph_comment_id: comment_id,
        },
        (),
        start,
    ))
}
