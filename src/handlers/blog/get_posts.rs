use std::{collections::HashMap, sync::Arc};

use crate::{
    domain::blog::blog::{PostInfo, PostInfoWithVote, UserBadgeInfo, VoteState},
    dto::{
        requests::blog::get_posts_request::GetPostsRequest,
        responses::{blog::get_posts::GetPostsResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{post_votes, user_profile_pictures, users},
    util::auth::is_superuser::is_superuser,
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

#[utoipa::path(
    get,
    path = "/api/blog/posts",
    tag = "blog",
    params(
        ("page" = Option<usize>, Query, description = "Page number"),
        ("posts_per_page" = Option<usize>, Query, description = "Posts per page")
    ),
    responses(
        (status = 200, description = "List of blog posts", body = GetPostsResponse),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_posts(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Query(request): Query<GetPostsRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let include_unpublished = match is_logged_in.clone() {
        AuthStatus::LoggedIn(user_id) => match is_superuser(state.clone(), user_id).await {
            Ok(is_superuser) => is_superuser,
            Err(e) => return Err(code_err(CodeError::DB_QUERY_ERROR, e)),
        },
        AuthStatus::LoggedOut => false,
    };

    let (post_infos, available_pages): (Vec<PostInfo>, usize) = state
        .get_posts_from_cache(request.page, request.posts_per_page, include_unpublished)
        .await;

    let post_ids: Vec<Uuid> = post_infos
        .iter()
        .map(|post| post.post_id)
        .collect::<Vec<Uuid>>();

    let mut user_ids: Vec<Uuid> = post_infos.iter().map(|post| post.user_id).collect();
    user_ids.sort();
    user_ids.dedup();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Fetch user names
    let authors: Vec<(Uuid, String)> = users::table
        .filter(users::user_id.eq_any(&user_ids))
        .select((users::user_id, users::user_name))
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let author_map: HashMap<Uuid, String> = authors.into_iter().collect();

    // Fetch profile pictures
    let author_pics: Vec<(Uuid, Option<String>)> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq_any(&user_ids))
        .order(user_profile_pictures::user_profile_picture_updated_at.desc())
        .select((
            user_profile_pictures::user_id,
            user_profile_pictures::user_profile_picture_link,
        ))
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let mut author_pic_map: HashMap<Uuid, String> = HashMap::new();
    for (uid, link) in author_pics {
        if !author_pic_map.contains_key(&uid)
            && let Some(l) = link
        {
            author_pic_map.insert(uid, l);
        }
    }

    let vote_map = if let AuthStatus::LoggedIn(user_id) = is_logged_in {
        let user_votes: Vec<(Uuid, bool)> = post_votes::table
            .filter(post_votes::post_id.eq_any(&post_ids))
            .filter(post_votes::user_id.eq(user_id))
            .select((post_votes::post_id, post_votes::is_upvote))
            .load::<(Uuid, bool)>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

        user_votes
            .into_iter()
            .map(|(pid, is_upvote)| {
                let state = if is_upvote {
                    VoteState::Upvoted
                } else {
                    VoteState::Downvoted
                };
                (pid, state)
            })
            .collect::<HashMap<Uuid, VoteState>>()
    } else {
        HashMap::new()
    };

    drop(conn);

    let posts: Vec<PostInfoWithVote> = post_infos
        .into_iter()
        .map(|post| {
            let vote_state = vote_map
                .get(&post.post_id)
                .cloned()
                .unwrap_or(VoteState::DidNotVote);

            let user_name = author_map
                .get(&post.user_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            let user_profile_picture_url = author_pic_map
                .get(&post.user_id)
                .cloned()
                .unwrap_or_default();

            PostInfoWithVote::from_info_with_vote(
                post,
                vote_state,
                UserBadgeInfo {
                    user_name,
                    user_profile_picture_url,
                },
            )
        })
        .collect();

    Ok(http_resp(
        GetPostsResponse {
            posts,
            available_pages,
        },
        (),
        start,
    ))
}
