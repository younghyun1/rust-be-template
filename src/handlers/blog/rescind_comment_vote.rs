use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{comment_votes::dsl as cu, comments},
    util::time::now::tokio_now,
};

#[derive(diesel::QueryableByName)]
struct VoteCounts {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    upvote_count: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    downvote_count: i64,
}

#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}/{comment_id}/vote",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post"),
        ("comment_id" = Uuid, Path, description = "ID of the comment to rescind vote for")
    ),
    responses(
        (status = 200, description = "Vote rescinded successfully"),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 404, description = "Vote does not exist", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn rescind_comment_vote(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path((_post_id, comment_id)): Path<(Uuid, Uuid)>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    match conn
        .transaction::<_, diesel::result::Error, _>(async |conn| {
            let affected_rows = diesel::delete(
                cu::comment_votes
                    .filter(cu::comment_id.eq(&comment_id).and(cu::user_id.eq(user_id))),
            )
            .execute(&mut *conn)
            .await?;

            if affected_rows == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            let counts: VoteCounts = diesel::sql_query(
                "SELECT \
                        COUNT(*) FILTER (WHERE is_upvote = true) AS upvote_count, \
                        COUNT(*) FILTER (WHERE is_upvote = false) AS downvote_count \
                     FROM comment_votes \
                     WHERE comment_id = $1",
            )
            .bind::<diesel::sql_types::Uuid, Uuid>(comment_id)
            .get_result(&mut *conn)
            .await?;

            diesel::update(comments::table.filter(comments::comment_id.eq(comment_id)))
                .set((
                    comments::total_upvotes.eq(counts.upvote_count),
                    comments::total_downvotes.eq(counts.downvote_count),
                ))
                .execute(&mut *conn)
                .await?;

            Ok(())
        })
        .await
    {
        Ok(()) => {}
        Err(diesel::result::Error::NotFound) => {
            return Err(CodeError::UPVOTE_DOES_NOT_EXIST.into());
        }
        Err(e) => return Err(code_err(CodeError::DB_DELETION_ERROR, e)),
    }

    Ok(http_resp((), (), start))
}
