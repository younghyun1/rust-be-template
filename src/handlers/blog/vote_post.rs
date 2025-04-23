use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    dto::{
        requests::blog::upvote_post_request::UpvotePostRequest,
        responses::{blog::vote_post_response::VotePostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[derive(Debug, diesel::QueryableByName)]
pub struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub upvote_count: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub downvote_count: i64,
}

pub async fn upvote_post(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpvotePostRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let count_row: CountRow = diesel::sql_query(
        "WITH ins AS (
             INSERT INTO post_upvotes (post_id, user_id, is_upvote)
             VALUES ($1, $2, $3)
             RETURNING 1
         )
         SELECT
             (SELECT count(*) FROM post_upvotes WHERE post_id = $1 AND is_upvote = true) AS upvote_count,
             (SELECT count(*) FROM post_upvotes WHERE post_id = $1 AND is_upvote = false) AS downvote_count
         ",
    )
    .bind::<diesel::sql_types::Uuid, Uuid>(request.post_id)
    .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
    .bind::<diesel::sql_types::Bool, bool>(request.is_upvote)
    .get_result(&mut conn)
    .await
    .map_err(|e| match e {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _error_info,
        ) => CodeError::UPVOTE_MUST_BE_UNIQUE.into(),
        e => code_err(CodeError::DB_INSERTION_ERROR, e),
    })?;

    // TODO: spawn off task here to update the denormalized count

    Ok(http_resp(
        VotePostResponse {
            upvote_count: count_row.upvote_count,
            downvote_count: count_row.downvote_count,
        },
        (),
        start,
    ))
}
