use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::{Comment, Post},
    dto::responses::{blog::read_post_response::ReadPostResponse, response_data::http_resp},
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{comments, posts},
    util::time::now::tokio_now,
};

// TODO: Get comments too.
pub async fn read_post(
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let post: Post = diesel::update(posts::table)
        .filter(posts::post_id.eq(post_id))
        .set(posts::post_view_count.eq(posts::post_view_count + 1))
        .returning(posts::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let comments: Vec<Comment> = comments::table
        .filter(comments::post_id.eq(post.post_id))
        .load::<Comment>(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    drop(conn);

    Ok(http_resp(ReadPostResponse { post, comments }, (), start))
}
