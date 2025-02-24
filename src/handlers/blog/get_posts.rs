use std::sync::Arc;

use crate::{
    domain::blog::Post,
    dto::requests::blog::get_posts_request::GetPostsRequest,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::posts,
    util::time::now::tokio_now,
};
use axum::{Json, extract::State, response::IntoResponse};
use diesel::{
    QueryDsl,
    query_builder::BoxedSelectStatement,
    sql_types::{Bool, Integer, Text, Uuid},
};
use diesel_async::RunQueryDsl;

// GET: /blog/{slug}
pub async fn get_posts(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<GetPostsRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let now = tokio_now();
    let conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    drop(conn);

    Ok(())
}
