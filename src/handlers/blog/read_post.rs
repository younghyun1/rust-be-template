use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::Comment,
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

    let post_handle = {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut conn = state
                .get_conn()
                .await
                .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

            diesel::update(posts::table)
                .filter(posts::post_id.eq(post_id))
                .set(posts::post_view_count.eq(posts::post_view_count + 1))
                .returning(posts::all_columns)
                .get_result(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))
        })
    };

    let comments_handle = {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut conn = state
                .get_conn()
                .await
                .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

            comments::table
                .filter(comments::post_id.eq(post_id))
                .load::<Comment>(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))
        })
    };

    let (post_result, comments_result) = tokio::join!(post_handle, comments_handle);

    let post = post_result.map_err(|e| code_err(CodeError::JOIN_ERROR, e))??;
    let comments = comments_result.map_err(|e| code_err(CodeError::JOIN_ERROR, e))??;

    // TODO: Grab updoots for these posts and comments

    Ok(http_resp(ReadPostResponse { post, comments }, (), start))
}
