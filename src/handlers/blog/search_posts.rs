use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension,
    extract::{Query, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde_derive::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    domain::blog::blog::{CachedPostInfo, PostInfoWithVote, UserBadgeInfo, VoteState},
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{post_votes, user_profile_pictures, users},
    util::time::now::tokio_now,
};

#[derive(Deserialize, IntoParams)]
pub struct SearchPostsRequest {
    /// The search query string
    pub q: String,
    /// Search type: "title" for title search, "tag" for tag search
    #[serde(default = "default_search_type")]
    pub search_type: String,
    /// Maximum number of results (default 20, max 100)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_search_type() -> String {
    "title".to_string()
}

fn default_limit() -> usize {
    20
}

#[derive(serde_derive::Serialize, ToSchema)]
pub struct SearchPostsResponse {
    pub posts: Vec<PostInfoWithVote>,
    pub query: String,
    pub search_type: String,
}

#[utoipa::path(
    get,
    path = "/api/blog/search",
    tag = "blog",
    params(SearchPostsRequest),
    responses(
        (status = 200, description = "Search results", body = SearchPostsResponse),
        (status = 400, description = "Invalid search parameters", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn search_posts(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Query(request): Query<SearchPostsRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let query = request.q.trim();
    if query.is_empty() {
        return Err(code_err(
            CodeError::INVALID_REQUEST,
            "Search query cannot be empty",
        ));
    }

    let limit = request.limit.clamp(1, 100);
    let search_type = request.search_type.to_lowercase();

    // Perform search based on type
    let matching_posts: Vec<CachedPostInfo> = match search_type.as_str() {
        "title" => state.search_posts_by_title(query, limit).await,
        "tag" => state.search_posts_by_tag(query, limit).await,
        _ => {
            return Err(code_err(
                CodeError::INVALID_REQUEST,
                "Invalid search_type. Use 'title' or 'tag'",
            ));
        }
    };

    if matching_posts.is_empty() {
        return Ok(http_resp(
            SearchPostsResponse {
                posts: vec![],
                query: query.to_string(),
                search_type,
            },
            (),
            start,
        ));
    }

    // Gather user IDs for author info
    let mut user_ids: Vec<Uuid> = matching_posts.iter().map(|p| p.user_id).collect();
    user_ids.sort();
    user_ids.dedup();

    let post_ids: Vec<Uuid> = matching_posts.iter().map(|p| p.post_id).collect();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Fetch user names and country codes
    let authors: Vec<(Uuid, String, i32)> = users::table
        .filter(users::user_id.eq_any(&user_ids))
        .select((users::user_id, users::user_name, users::user_country))
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let mut author_map: HashMap<Uuid, String> = HashMap::new();
    let mut author_country_map: HashMap<Uuid, i32> = HashMap::new();
    for (uid, name, country) in authors {
        author_map.insert(uid, name);
        author_country_map.insert(uid, country);
    }

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

    // Fetch vote states if logged in
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

    // Get country flag lookup from cache
    let country_map = state.country_map.read().await;

    let posts: Vec<PostInfoWithVote> = matching_posts
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
            let user_country_flag = author_country_map
                .get(&post.user_id)
                .and_then(|&code| country_map.get_flag_by_code(code));

            PostInfoWithVote::from_cached_info_with_vote(
                post,
                vote_state,
                UserBadgeInfo {
                    user_name,
                    user_profile_picture_url,
                    user_country_flag,
                },
            )
        })
        .collect();

    drop(country_map);

    Ok(http_resp(
        SearchPostsResponse {
            posts,
            query: query.to_string(),
            search_type,
        },
        (),
        start,
    ))
}
