use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::{
    domain::blog::blog::PostInfo,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{post_votes, posts},
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
    path = "/api/blog/{post_id}/vote",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to rescind vote for")
    ),
    responses(
        (status = 200, description = "Vote rescinded successfully"),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 404, description = "Post or vote not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn rescind_post_vote(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let mut post_info: PostInfo = match state.blog_posts_cache.get_async(&post_id).await {
        Some(post_info) => post_info.clone(),
        None => {
            return Err(code_err(
                CodeError::POST_NOT_FOUND_IN_CACHE,
                "Post not found",
            ));
        }
    };

    let (upvote_count, downvote_count) = match conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                let affected_rows = diesel::delete(
                    post_votes::table.filter(
                        post_votes::post_id
                            .eq(post_id)
                            .and(post_votes::user_id.eq(user_id)),
                    ),
                )
                .execute(conn)
                .await?;

                if affected_rows == 0 {
                    return Err(diesel::result::Error::NotFound);
                }

                let counts: VoteCounts = diesel::sql_query(
                    "SELECT \
                        COUNT(*) FILTER (WHERE is_upvote = true) AS upvote_count, \
                        COUNT(*) FILTER (WHERE is_upvote = false) AS downvote_count \
                     FROM post_votes \
                     WHERE post_id = $1",
                )
                .bind::<diesel::sql_types::Uuid, Uuid>(post_id)
                .get_result(conn)
                .await?;

                diesel::update(posts::table.filter(posts::post_id.eq(post_id)))
                    .set((
                        posts::total_upvotes.eq(counts.upvote_count),
                        posts::total_downvotes.eq(counts.downvote_count),
                    ))
                    .execute(conn)
                    .await?;

                Ok((counts.upvote_count, counts.downvote_count))
            })
        })
        .await
    {
        Ok(counts) => counts,
        Err(diesel::result::Error::NotFound) => return Err(CodeError::UPVOTE_DOES_NOT_EXIST.into()),
        Err(e) => return Err(code_err(CodeError::DB_DELETION_ERROR, e)),
    };

    post_info.total_upvotes = upvote_count;
    post_info.total_downvotes = downvote_count;

    match state
        .blog_posts_cache
        .update_async(&post_id, |_, cached_post_info| {
            *cached_post_info = post_info.clone();
        })
        .await
    {
        Some(_) => (),
        None => {
            return Err(code_err(
                CodeError::POST_CACHE_INSERTION_ERROR,
                format!("Could not insert post with ID {}", post_id),
            ));
        }
    };

    Ok(http_resp((), (), start))
}
