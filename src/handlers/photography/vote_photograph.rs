//! `POST /api/photographs/{photograph_id}/vote` — upsert the caller's vote.
//!
//! Mirrors `vote_post`: raw `ON CONFLICT` upsert, FILTER recount, denormalized
//! counter update, all in one transaction.

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
    schema::photographs,
    util::time::now::tokio_now,
};

#[utoipa::path(
    post,
    path = "/api/photographs/{photograph_id}/vote",
    tag = "photography",
    params(("photograph_id" = Uuid, Path, description = "Photograph to vote on")),
    request_body = VotePhotographRequest,
    responses(
        (status = 200, description = "Vote recorded", body = VotePhotographResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn vote_photograph(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(photograph_id): Path<Uuid>,
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
                "INSERT INTO photograph_votes (photograph_id, user_id, is_upvote)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (photograph_id, user_id)
                     DO UPDATE SET is_upvote = EXCLUDED.is_upvote",
            )
            .bind::<diesel::sql_types::Uuid, Uuid>(photograph_id)
            .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
            .bind::<diesel::sql_types::Bool, bool>(is_upvote)
            .execute(&mut *conn)
            .await?;

            let counts: VoteCounts = diesel::sql_query(
                "SELECT \
                        COUNT(*) FILTER (WHERE is_upvote = true) AS upvote_count, \
                        COUNT(*) FILTER (WHERE is_upvote = false) AS downvote_count \
                     FROM photograph_votes \
                     WHERE photograph_id = $1",
            )
            .bind::<diesel::sql_types::Uuid, Uuid>(photograph_id)
            .get_result(&mut *conn)
            .await?;

            diesel::update(photographs::table.filter(photographs::photograph_id.eq(photograph_id)))
                .set((
                    photographs::photograph_total_upvotes.eq(counts.upvote_count),
                    photographs::photograph_total_downvotes.eq(counts.downvote_count),
                ))
                .execute(&mut *conn)
                .await?;

            Ok(counts)
        })
        .await
    {
        Ok(counts) => counts,
        Err(e) => match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => return Err(CodeError::UPVOTE_MUST_BE_UNIQUE.into()),
            e => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
        },
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
