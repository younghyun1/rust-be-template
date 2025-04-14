use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::NewCommentVote,
    dto::{
        requests::blog::upvote_comment_request::UpvoteCommentRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::comment_votes,
    util::time::now::tokio_now,
};

pub async fn upvote_comment(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpvoteCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let new_comment_upvote: NewCommentVote =
        NewCommentVote::new(&request.comment_id, &user_id, true);

    match diesel::insert_into(comment_votes::table)
        .values(new_comment_upvote)
        .execute(&mut conn)
        .await
    {
        Ok(_) => (),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => return Err(CodeError::UPVOTE_MUST_BE_UNIQUE.into()),
        Err(e) => return Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
    };

    Ok(http_resp((), (), start))
}
