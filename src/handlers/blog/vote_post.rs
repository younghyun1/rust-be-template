use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::{
    domain::blog::blog::CachedPostInfo,
    dto::{
        requests::blog::upvote_post_request::UpvotePostRequest,
        responses::{blog::vote_post_response::VotePostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::posts,
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
    post,
    path = "/api/blog/{post_id}/vote",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to vote for")
    ),
    request_body = UpvotePostRequest,
    responses(
        (status = 200, description = "Vote recorded", body = VotePostResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 404, description = "Post not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn vote_post(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
    Json(request): Json<UpvotePostRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let mut post_info: CachedPostInfo = match state.blog_posts_cache.get_async(&post_id).await {
        Some(post_info) => post_info.clone(),
        None => {
            return Err(code_err(
                CodeError::POST_NOT_FOUND_IN_CACHE,
                "Post not found",
            ));
        }
    };

    let (upvote_count, downvote_count): (i64, i64) = match conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            let is_upvote = request.is_upvote;

            Box::pin(async move {
                // 1. Insert or update the vote
                diesel::sql_query(
                    "INSERT INTO post_votes (post_id, user_id, is_upvote)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (post_id, user_id)
                     DO UPDATE SET is_upvote = EXCLUDED.is_upvote",
                )
                .bind::<diesel::sql_types::Uuid, Uuid>(post_id)
                .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
                .bind::<diesel::sql_types::Bool, bool>(is_upvote)
                .execute(conn)
                .await?;

                // 2. Get both counts in a single query
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

                // 3. Update both columns in the posts table at once
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
        Ok(tuple) => tuple,
        Err(e) => match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _error_info,
            ) => return Err(CodeError::UPVOTE_MUST_BE_UNIQUE.into()),
            e => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
        },
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

    Ok(http_resp(
        VotePostResponse {
            upvote_count,
            downvote_count,
            is_upvote: request.is_upvote,
        },
        (),
        start,
    ))
}
