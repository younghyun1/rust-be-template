use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    dto::{
        requests::blog::upvote_comment_request::UpvoteCommentRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::comment_upvotes::dsl as cu,
    util::time::now::tokio_now,
};

pub async fn rescind_comment_vote(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpvoteCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let affected_rows = diesel::delete(
        cu::comment_upvotes.filter(
            cu::comment_id
                .eq(&request.comment_id)
                .and(cu::user_id.eq(user_id)),
        ),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| code_err(CodeError::DB_DELETION_ERROR, e))?;

    if affected_rows == 0 {
        return Err(CodeError::UPVOTE_DOES_NOT_EXIST.into());
    }

    Ok(http_resp((), (), start))
}
