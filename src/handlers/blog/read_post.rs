use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::blog::{Comment, CommentResponse, UserBadgeInfo, VoteState},
    dto::responses::{blog::read_post_response::ReadPostResponse, response_data::http_resp},
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{comment_votes, comments, post_votes, posts, user_profile_pictures, users},
    util::time::now::tokio_now,
};

// TODO: Get comments too.
#[utoipa::path(
    get,
    path = "/api/blog/posts/{post_id}",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to read")
    ),
    responses(
        (status = 200, description = "Post details and comments", body = ReadPostResponse),
        (status = 404, description = "Post not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
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

    let mut relevant_user_ids: Vec<Uuid> = comments.iter().map(|c| c.user_id).collect();
    relevant_user_ids.push(post.user_id);
    relevant_user_ids.sort();
    relevant_user_ids.dedup();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Fetch user names
    let users_info: Vec<(Uuid, String)> = users::table
        .filter(users::user_id.eq_any(&relevant_user_ids))
        .select((users::user_id, users::user_name))
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let user_name_map: HashMap<Uuid, String> = users_info.into_iter().collect();

    // Fetch profile pictures
    let user_pics: Vec<(Uuid, Option<String>)> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq_any(&relevant_user_ids))
        .order(user_profile_pictures::user_profile_picture_updated_at.desc())
        .select((
            user_profile_pictures::user_id,
            user_profile_pictures::user_profile_picture_link,
        ))
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let mut user_pic_map: HashMap<Uuid, String> = HashMap::new();
    for (uid, link) in user_pics {
        if !user_pic_map.contains_key(&uid)
            && let Some(l) = link
        {
            user_pic_map.insert(uid, l);
        }
    }

    drop(conn);

    // Fetch vote state for comments if logged in
    let vote_map = if let AuthStatus::LoggedIn(user_id) = is_logged_in {
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

        user_votes
            .into_iter()
            .map(|(cid, is_upvote)| {
                let vs = if is_upvote {
                    VoteState::Upvoted
                } else {
                    VoteState::Downvoted
                };
                (cid, vs)
            })
            .collect::<HashMap<Uuid, VoteState>>()
    } else {
        HashMap::new()
    };

    // Transform comments into CommentResponse
    let mut comment_responses: Vec<CommentResponse> = comments
        .into_iter()
        .map(|comment| {
            let vs = vote_map
                .get(&comment.comment_id)
                .cloned()
                .unwrap_or(VoteState::DidNotVote);

            let user_name = user_name_map
                .get(&comment.user_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            let user_profile_picture_url = user_pic_map
                .get(&comment.user_id)
                .cloned()
                .unwrap_or_default();

            CommentResponse::from_comment_votestate_and_badge_info(
                comment,
                vs,
                UserBadgeInfo {
                    user_name,
                    user_profile_picture_url,
                },
            )
        })
        .collect();

    comment_responses.sort_by_key(|c| -(c.total_upvotes - c.total_downvotes));

    let post_author_name = user_name_map
        .get(&post.user_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let post_author_pic = user_pic_map.get(&post.user_id).cloned().unwrap_or_default();

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
            user_badge_info: UserBadgeInfo {
                user_name: post_author_name,
                user_profile_picture_url: post_author_pic,
            },
        },
        (),
        start,
    ))
}
