//! `GET /api/photographs/{photograph_id}` — public detail endpoint.
//!
//! Increments the naive view count (+1 per call), returns the photograph row
//! (with denormalized view/vote counts), the caller's vote state, and the
//! enriched flat comment list (threaded client-side via parent ids). Mirrors the
//! blog `read_post` enrichment. Public/200 like `read_post`: never 401/403, so
//! the frontend's session guard is not tripped on open.

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
    domain::{
        blog::blog::{UserBadgeInfo, VoteState},
        photography::{
            photographs::Photograph,
            social::{PhotographComment, PhotographCommentResponse},
        },
    },
    dto::responses::{
        photography::read_photograph_response::ReadPhotographResponse, response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{
        photograph_comment_votes, photograph_comments, photograph_votes, photographs,
        user_profile_pictures, users,
    },
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/photographs/{photograph_id}",
    tag = "photography",
    params(("photograph_id" = Uuid, Path, description = "Photograph id")),
    responses(
        (status = 200, description = "Photograph detail", body = ReadPhotographResponse),
        (status = 404, description = "Photograph not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn read_photograph(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Path(photograph_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Naive +1 view, return the updated row.
    let photograph: Photograph =
        diesel::update(photographs::table.filter(photographs::photograph_id.eq(photograph_id)))
            .set(photographs::photograph_view_count.eq(photographs::photograph_view_count + 1))
            .returning(photographs::all_columns)
            .get_result(&mut conn)
            .await
            .optional()
            .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?
            .ok_or_else(|| code_err(CodeError::PHOTOGRAPH_NOT_FOUND, "Photograph not found"))?;

    let comments: Vec<PhotographComment> = photograph_comments::table
        .filter(photograph_comments::photograph_id.eq(photograph_id))
        .load::<PhotographComment>(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    // Author + commenters.
    let mut relevant_user_ids: Vec<Uuid> = comments.iter().map(|c| c.user_id).collect();
    relevant_user_ids.push(photograph.user_id);
    relevant_user_ids.sort();
    relevant_user_ids.dedup();

    let users_info: Vec<(Uuid, String, i32)> = users::table
        .filter(users::user_id.eq_any(&relevant_user_ids))
        .select((users::user_id, users::user_name, users::user_country))
        .load(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let mut user_name_map: HashMap<Uuid, String> = HashMap::new();
    let mut user_country_map: HashMap<Uuid, i32> = HashMap::new();
    for (uid, name, country) in users_info {
        user_name_map.insert(uid, name);
        user_country_map.insert(uid, country);
    }

    let user_pics: Vec<(Uuid, Option<String>)> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq_any(&relevant_user_ids))
        .distinct_on(user_profile_pictures::user_id)
        .order((
            user_profile_pictures::user_id,
            user_profile_pictures::user_profile_picture_updated_at.desc(),
        ))
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

    // Per-comment vote state for the caller.
    let comment_vote_map: HashMap<Uuid, VoteState> =
        if let AuthStatus::LoggedIn(user_id) = is_logged_in {
            let comment_ids: Vec<Uuid> = comments.iter().map(|c| c.photograph_comment_id).collect();
            let user_votes: Vec<(Uuid, bool)> = photograph_comment_votes::table
                .filter(photograph_comment_votes::photograph_comment_id.eq_any(&comment_ids))
                .filter(photograph_comment_votes::user_id.eq(user_id))
                .select((
                    photograph_comment_votes::photograph_comment_id,
                    photograph_comment_votes::is_upvote,
                ))
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
                .collect()
        } else {
            HashMap::new()
        };

    // Caller's vote state for the photograph itself.
    let photograph_vote_state = if let AuthStatus::LoggedIn(user_id) = is_logged_in {
        let opt = photograph_votes::table
            .filter(photograph_votes::photograph_id.eq(photograph_id))
            .filter(photograph_votes::user_id.eq(user_id))
            .select(photograph_votes::is_upvote)
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

    drop(conn);

    let country_map = state.country_map.read().await;

    let badge_for = |uid: &Uuid| UserBadgeInfo {
        user_name: user_name_map
            .get(uid)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        user_profile_picture_url: user_pic_map.get(uid).cloned().unwrap_or_default(),
        user_country_flag: user_country_map
            .get(uid)
            .and_then(|&code| country_map.get_flag_by_code(code)),
    };

    let mut comment_responses: Vec<PhotographCommentResponse> = comments
        .into_iter()
        .map(|comment| {
            let vs = comment_vote_map
                .get(&comment.photograph_comment_id)
                .cloned()
                .unwrap_or(VoteState::DidNotVote);
            let badge = badge_for(&comment.user_id);
            PhotographCommentResponse::from_comment_votestate_and_badge_info(comment, vs, badge)
        })
        .collect();
    comment_responses.sort_by_key(|c| {
        -(c.photograph_comment_total_upvotes - c.photograph_comment_total_downvotes)
    });

    let author_badge = badge_for(&photograph.user_id);
    drop(country_map);

    Ok(http_resp(
        ReadPhotographResponse {
            photograph,
            vote_state: photograph_vote_state,
            comments: comment_responses,
            user_badge_info: author_badge,
        },
        (),
        start,
    ))
}
