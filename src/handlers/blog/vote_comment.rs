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
    errors::code_error::{CodeError, HandlerResponse, code_err},
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
                let count_row: CountRow = diesel::sql_query(
                    "WITH ins AS (
                        INSERT INTO comment_votes (comment_id, user_id, is_upvote)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (comment_id, user_id) DO UPDATE SET is_upvote = EXCLUDED.is_upvote
                        RETURNING 1
                    )
                    SELECT
                        (SELECT count(*) FROM comment_votes WHERE comment_id = $1 AND is_upvote = true) AS upvote_count,
                        (SELECT count(*) FROM comment_votes WHERE comment_id = $1 AND is_upvote = false) AS downvote_count
                    ",
                )
                .bind::<diesel::sql_types::Uuid, Uuid>(comment_id)
                .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
                .bind::<diesel::sql_types::Bool, bool>(is_upvote)
                .get_result(conn)
                .await?;

                diesel::update(comments::table.filter(comments::comment_id.eq(comment_id)))
                    .set(comments::total_upvotes.eq(count_row.upvote_count))
                    .execute(conn)
                    .await?;
                diesel::update(comments::table.filter(comments::comment_id.eq(comment_id)))
                    .set(comments::total_downvotes.eq(count_row.downvote_count))
                    .execute(conn)
                    .await?;

                Ok(count_row)
            })
        })
        .await
    {
        Ok(crow) => crow,
        Err(e) => {
            match e {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => return Err(CodeError::UPVOTE_MUST_BE_UNIQUE.into()),
                e => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
            }
        }
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
