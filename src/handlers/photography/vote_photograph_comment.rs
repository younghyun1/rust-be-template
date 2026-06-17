//! `POST /api/photographs/{photograph_id}/{comment_id}/vote` — vote a comment.

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
    domain::photography::social::VoteCounts,
    dto::{
        requests::photography::vote_photograph_request::VotePhotographRequest,
        responses::{
            photography::vote_photograph_response::VotePhotographResponse, response_data::http_resp,
        },
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::photograph_comments,
    util::time::now::tokio_now,
};

#[utoipa::path(
    post,
    path = "/api/photographs/{photograph_id}/{comment_id}/vote",
    tag = "photography",
    params(
        ("photograph_id" = Uuid, Path, description = "Photograph id"),
        ("comment_id" = Uuid, Path, description = "Comment to vote on")
    ),
    request_body = VotePhotographRequest,
    responses(
        (status = 200, description = "Vote recorded", body = VotePhotographResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn vote_photograph_comment(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path((_photograph_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<VotePhotographRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let counts: VoteCounts = match conn
        .transaction::<_, diesel::result::Error, _>(async |conn| {
            let is_upvote = request.is_upvote;

            diesel::sql_query(
                "INSERT INTO photograph_comment_votes (photograph_comment_id, user_id, is_upvote)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (photograph_comment_id, user_id)
                     DO UPDATE SET is_upvote = EXCLUDED.is_upvote",
            )
            .bind::<diesel::sql_types::Uuid, Uuid>(comment_id)
            .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
            .bind::<diesel::sql_types::Bool, bool>(is_upvote)
            .execute(&mut *conn)
            .await?;

            let counts: VoteCounts = diesel::sql_query(
                "SELECT \
                        COUNT(*) FILTER (WHERE is_upvote = true) AS upvote_count, \
                        COUNT(*) FILTER (WHERE is_upvote = false) AS downvote_count \
                     FROM photograph_comment_votes \
                     WHERE photograph_comment_id = $1",
            )
            .bind::<diesel::sql_types::Uuid, Uuid>(comment_id)
            .get_result(&mut *conn)
            .await?;

            diesel::update(
                photograph_comments::table
                    .filter(photograph_comments::photograph_comment_id.eq(comment_id)),
            )
            .set((
                photograph_comments::photograph_comment_total_upvotes.eq(counts.upvote_count),
                photograph_comments::photograph_comment_total_downvotes.eq(counts.downvote_count),
            ))
            .execute(&mut *conn)
            .await?;

            Ok(counts)
        })
        .await
    {
        Ok(counts) => counts,
        Err(e) => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
    };

    Ok(http_resp(
        VotePhotographResponse {
            upvote_count: counts.upvote_count,
            downvote_count: counts.downvote_count,
            is_upvote: request.is_upvote,
        },
        (),
        start,
    ))
}
