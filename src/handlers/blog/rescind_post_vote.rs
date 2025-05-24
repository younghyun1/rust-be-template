use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    dto::{
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::post_votes::dsl as pu,
    util::time::now::tokio_now,
};

pub async fn rescind_post_vote(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let affected_rows = diesel::delete(
        pu::post_votes.filter(pu::post_id.eq(&post_id).and(pu::user_id.eq(user_id))),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| code_err(CodeError::DB_DELETION_ERROR, e))?;

    if affected_rows == 0 {
        return Err(CodeError::UPVOTE_DOES_NOT_EXIST.into());
    }

    Ok(http_resp((), (), start))
}
