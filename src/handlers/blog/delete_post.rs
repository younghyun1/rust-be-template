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
    domain::auth::role::RoleType,
    dto::responses::{blog::delete_post_response::DeletePostResponse, response_data::http_resp},
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::posts,
    util::time::now::tokio_now,
};

#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to delete")
    ),
    responses(
        (status = 200, description = "Post deleted successfully", body = DeletePostResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden", body = CodeErrorResp),
        (status = 404, description = "Post not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn delete_post(
    Extension(requester_id): Extension<Uuid>,
    Extension(role_type): Extension<RoleType>,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let is_superuser = role_type.is_superuser();

    // 1. Check post author against requester ID.
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let author_id: Uuid = posts::table
        .select(posts::user_id)
        .filter(posts::post_id.eq(post_id))
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

    // delete from state
    state.delete_post_from_cache(post_id).await;

    Ok(http_resp(
        DeletePostResponse {
            deleted_post_id: post_id,
        },
        (),
        start,
    ))
}
