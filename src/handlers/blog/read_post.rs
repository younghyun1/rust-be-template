use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::blog::{Comment, CommentResponse, VoteState},
    dto::responses::{blog::read_post_response::ReadPostResponse, response_data::http_resp},
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{comment_votes, comments, post_votes, posts},
    util::time::now::tokio_now,
};

// TODO: Get comments too.
pub async fn read_post(
    Extension(is_logged_in): Extension<AuthStatus>,
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

    let post: crate::domain::blog::blog::Post =
        post_result.map_err(|e| code_err(CodeError::JOIN_ERROR, e))??;

    let comments: Vec<Comment> =
        comments_result.map_err(|e| code_err(CodeError::JOIN_ERROR, e))??;

    // Transform comments into CommentResponse with vote state
    let mut comment_responses = if let AuthStatus::LoggedIn(user_id) = is_logged_in {
        let comment_ids: Vec<Uuid> = comments.iter().map(|c| c.comment_id).collect();
        let mut conn = state
            .get_conn()
            .await
            .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

        let user_votes: Vec<(Uuid, bool)> = comment_votes::table
            .filter(comment_votes::comment_id.eq_any(&comment_ids))
            .filter(comment_votes::user_id.eq(user_id))
            .select((comment_votes::comment_id, comment_votes::is_upvote))
            .load::<(Uuid, bool)>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

        let vote_map: std::collections::HashMap<Uuid, VoteState> = user_votes
            .into_iter()
            .map(|(cid, is_upvote)| {
                let vs = if is_upvote {
                    VoteState::Upvoted
                } else {
                    VoteState::Downvoted
                };
                (cid, vs)
            })
            .collect();

        comments
            .into_iter()
            .map(|comment| {
                let vs = vote_map
                    .get(&comment.comment_id)
                    .cloned()
                    .unwrap_or(VoteState::DidNotVote);
                CommentResponse::from_comment_and_votestate(comment, vs)
            })
            .collect::<Vec<_>>()
    } else {
        comments
            .into_iter()
            .map(|comment| {
                CommentResponse::from_comment_and_votestate(comment, VoteState::DidNotVote)
            })
            .collect::<Vec<_>>()
    };

    comment_responses.sort_by_key(|c| -(c.total_upvotes - c.total_downvotes));

    let post_vote_state = if let AuthStatus::LoggedIn(user_id) = is_logged_in {
        let mut conn = state
            .get_conn()
            .await
            .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;
        let opt = post_votes::table
            .filter(post_votes::post_id.eq(post_id))
            .filter(post_votes::user_id.eq(user_id))
            .select(post_votes::is_upvote)
            .first::<bool>(&mut conn)
            .await
            .optional()
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;
        match opt {
            Some(true) => VoteState::Upvoted,
            Some(false) => VoteState::Downvoted,
            None => VoteState::DidNotVote,
        }
    } else {
        VoteState::DidNotVote
    };

    Ok(http_resp(
        ReadPostResponse {
            post,
            comments: comment_responses,
            vote_state: post_vote_state,
        },
        (),
        start,
    ))
}
