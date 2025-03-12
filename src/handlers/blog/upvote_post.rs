use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    dto::{
        requests::blog::upvote_post_request::UpvotePostRequest, responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[derive(Debug, diesel::QueryableByName)]
pub struct CountRow {
    #[sql_type = "diesel::sql_types::BigInt"]
    count: i64,
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
             INSERT INTO post_upvotes (post_id, user_id)
             VALUES ($1, $2)
             RETURNING 1
         )
         SELECT count(*) as count FROM ins",
    )
    .bind::<diesel::sql_types::Uuid, _>(request.post_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .get_result(&mut conn)
    .await
    .map_err(|e| match e {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => CodeError::UPVOTE_MUST_BE_UNIQUE.into(),
        e => code_err(CodeError::DB_INSERTION_ERROR, e),
    })?;

    Ok(http_resp(count_row.count, (), start))
}
