use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::{
    dto::{
        requests::blog::upvote_post_request::UpvotePostRequest,
        responses::{blog::vote_post_response::VotePostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::posts,
    util::time::now::tokio_now,
};

#[derive(Debug, diesel::QueryableByName)]
pub struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub upvote_count: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub downvote_count: i64,
}

pub async fn vote_post(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpvotePostRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let count_row: CountRow = match conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            let post_id = request.post_id;
            let is_upvote = request.is_upvote;

            Box::pin(async move {
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
                .bind::<diesel::sql_types::Uuid, Uuid>(post_id)
                .bind::<diesel::sql_types::Uuid, Uuid>(user_id)
                .bind::<diesel::sql_types::Bool, bool>(is_upvote)
                .get_result(conn)
                .await?;

                if is_upvote {
                    diesel::update(posts::table.filter(posts::post_id.eq(post_id)))
                        .set(posts::total_upvotes.eq(count_row.upvote_count))
                        .execute(conn)
                        .await?;
                } else {
                    diesel::update(posts::table.filter(posts::post_id.eq(post_id)))
                        .set(posts::total_downvotes.eq(count_row.downvote_count))
                        .execute(conn)
                        .await?;
                }

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
                    _error_info,
                ) => return Err(CodeError::UPVOTE_MUST_BE_UNIQUE.into()),
                e => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
            }
        }
    };

    Ok(http_resp(
        VotePostResponse {
            upvote_count: count_row.upvote_count,
            downvote_count: count_row.downvote_count,
        },
        (),
        start,
    ))
}
