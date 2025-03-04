use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;

use crate::{
    domain::blog::Post,
    dto::{requests::blog::read_post::ReadPostRequest, responses::response_data::http_resp},
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::posts,
    util::time::now::tokio_now,
};

// TODO: Iterate view count any time this happens.
pub async fn read_post(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ReadPostRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let post: Post = diesel::update(posts::table)
        .filter(posts::post_id.eq(request.post_id))
        .set(posts::post_view_count.eq(posts::post_view_count + 1))
        .returning(posts::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    drop(conn);

    Ok(http_resp(post, (), start))
}
