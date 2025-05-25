use std::{collections::HashMap, sync::Arc};

use crate::{
    domain::blog::blog::{PostInfo, PostInfoWithVote, VoteState},
    dto::{
        requests::blog::get_posts_request::GetPostsRequest,
        responses::{blog::get_posts::GetPostsResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::post_votes,
    util::time::now::tokio_now,
};
use axum::{
    Extension,
    extract::{Query, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

/// GET /blog/get-posts
/// Get posts metadata for post list.
pub async fn get_posts(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Query(request): Query<GetPostsRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let (post_infos, available_pages): (Vec<PostInfo>, usize) = state
        .get_posts_from_cache(request.page, request.posts_per_page)
        .await;

    let post_ids: Vec<Uuid> = post_infos
        .iter()
        .map(|post| post.post_id)
        .collect::<Vec<Uuid>>();

    let posts: Vec<PostInfoWithVote> = match is_logged_in {
        AuthStatus::LoggedIn(user_id) => {
            let mut conn = state
                .get_conn()
                .await
                .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

            // Fetch the user's votes for all relevant post_ids
            let user_votes: Vec<(Uuid, bool)> = post_votes::table
                .filter(post_votes::post_id.eq_any(&post_ids))
                .filter(post_votes::user_id.eq(user_id))
                .select((post_votes::post_id, post_votes::is_upvote))
                .load::<(Uuid, bool)>(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

            // Map to HashMap<Uuid, VoteState>
            let vote_map: HashMap<Uuid, VoteState> = user_votes
                .into_iter()
                .map(|(pid, is_upvote)| {
                    let state = if is_upvote {
                        VoteState::Upvoted
                    } else {
                        VoteState::Downvoted
                    };
                    (pid, state)
                })
                .collect();

            drop(conn);

            post_infos
                .into_iter()
                .map(|post| {
                    let vote_state = vote_map
                        .get(&post.post_id)
                        .cloned()
                        .unwrap_or(VoteState::DidNotVote);
                    PostInfoWithVote::from_info_with_vote(post, vote_state)
                })
                .collect()
        }
        AuthStatus::LoggedOut => post_infos
            .iter()
            .map(|post| PostInfoWithVote::from_info_with_vote(post.clone(), VoteState::DidNotVote))
            .collect(),
    };

    Ok(http_resp(
        GetPostsResponse {
            posts,
            available_pages,
        },
        (),
        start,
    ))
}
