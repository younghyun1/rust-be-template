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
    dto::{
        requests::blog::upvote_comment_request::UpvoteCommentRequest,
        responses::{blog::vote_comment_response::VoteCommentResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::comments,
    util::time::now::tokio_now,
};

#[derive(Debug, diesel::QueryableByName)]
pub struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub upvote_count: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub downvote_count: i64,
}
#[utoipa::path(
    post,
    path = "/api/blog/{post_id}/{comment_id}/vote",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post"),
        ("comment_id" = Uuid, Path, description = "ID of the comment to vote for")
    ),
    request_body = UpvoteCommentRequest,
    responses(
        (status = 200, description = "Vote recorded", body = VoteCommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn vote_comment(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path((_post_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpvoteCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let count_row: CountRow = match conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            let is_upvote = request.is_upvote;

            Box::pin(async move {
                // 1. Insert or update the vote
                diesel::sql_query(
                    "INSERT INTO comment_votes (comment_id, user_id, is_upvote)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (comment_id, user_id)
                     DO UPDATE SET is_upvote = EXCLUDED.is_upvote",
                )
                .bind::<diesel::sql_types::Uuid, Uuid>(comment_id)
                .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
                .bind::<diesel::sql_types::Bool, bool>(is_upvote)
                .execute(conn)
                .await?;

                // 2. Get new counts
                let counts: CountRow = diesel::sql_query(
                    "SELECT \
                        COUNT(*) FILTER (WHERE is_upvote = true) AS upvote_count, \
                        COUNT(*) FILTER (WHERE is_upvote = false) AS downvote_count \
                     FROM comment_votes \
                     WHERE comment_id = $1",
                )
                .bind::<diesel::sql_types::Uuid, Uuid>(comment_id)
                .get_result(conn)
                .await?;

                // 3. Update the comments table with the new counts
                diesel::update(comments::table.filter(comments::comment_id.eq(comment_id)))
                    .set((
                        comments::total_upvotes.eq(counts.upvote_count),
                        comments::total_downvotes.eq(counts.downvote_count),
                    ))
                    .execute(conn)
                    .await?;

                Ok(counts)
            })
        })
        .await
    {
        Ok(crow) => crow,
        Err(e) => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)), // Simplified error handling
    };

    Ok(http_resp(
        VoteCommentResponse {
            upvote_count: count_row.upvote_count,
            downvote_count: count_row.downvote_count,
            is_upvote: request.is_upvote,
        },
        (),
        start,
    ))
}
