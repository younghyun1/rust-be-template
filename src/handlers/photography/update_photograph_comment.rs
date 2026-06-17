//! `PATCH /api/photographs/{photograph_id}/{comment_id}` — edit a comment.
//! Author or superuser only.

use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::{
        auth::role::RoleType,
        blog::blog::{UserBadgeInfo, VoteState},
        photography::social::{PhotographComment, PhotographCommentResponse},
    },
    dto::{
        requests::photography::update_photograph_comment_request::UpdatePhotographCommentRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{photograph_comment_votes, photograph_comments, user_profile_pictures, users},
    util::time::now::tokio_now,
};

#[utoipa::path(
    patch,
    path = "/api/photographs/{photograph_id}/{comment_id}",
    tag = "photography",
    params(
        ("photograph_id" = Uuid, Path, description = "Photograph id"),
        ("comment_id" = Uuid, Path, description = "Comment to edit")
    ),
    request_body = UpdatePhotographCommentRequest,
    responses(
        (status = 200, description = "Comment updated", body = PhotographCommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 404, description = "Comment not found", body = CodeErrorResp)
    )
)]
pub async fn update_photograph_comment(
    Extension(requester_id): Extension<Uuid>,
    Extension(role_type): Extension<RoleType>,
    State(state): State<Arc<ServerState>>,
    Path((_photograph_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdatePhotographCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    if request.comment_content.trim().is_empty() {
        return Err(code_err(
            CodeError::INVALID_REQUEST,
            "Comment content must not be empty",
        ));
    }

    let is_superuser = role_type.is_superuser();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let author_id: Uuid = photograph_comments::table
        .select(photograph_comments::user_id)
        .filter(photograph_comments::photograph_comment_id.eq(comment_id))
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
        .ok_or_else(|| code_err(CodeError::COMMENT_NOT_FOUND, "Comment not found"))?;

    if author_id != requester_id && !is_superuser {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "User is not authorized to edit this comment",
        ));
    }

    let updated: PhotographComment = diesel::update(
        photograph_comments::table
            .filter(photograph_comments::photograph_comment_id.eq(comment_id)),
    )
    .set((
        photograph_comments::photograph_comment_content.eq(&request.comment_content),
        photograph_comments::photograph_comment_updated_at.eq(chrono::Utc::now()),
    ))
    .returning(photograph_comments::all_columns)
    .get_result(&mut conn)
    .await
    .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?;

    // Caller's current vote on this comment (for the returned vote_state).
    let vote_opt = photograph_comment_votes::table
        .filter(photograph_comment_votes::photograph_comment_id.eq(comment_id))
        .filter(photograph_comment_votes::user_id.eq(requester_id))
        .select(photograph_comment_votes::is_upvote)
        .first::<bool>(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;
    let vote_state = match vote_opt {
        Some(true) => VoteState::Upvoted,
        Some(false) => VoteState::Downvoted,
        None => VoteState::DidNotVote,
    };

    // Badge of the comment's author (not necessarily the requester).
    let author_uid = updated.user_id;
    let user_row: Option<(String, i32)> = users::table
        .filter(users::user_id.eq(author_uid))
        .select((users::user_name, users::user_country))
        .first::<(String, i32)>(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;
    let (user_name, user_country) = user_row.unwrap_or_else(|| ("Unknown".to_string(), 0));

    let pic: Option<String> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq(author_uid))
        .order(user_profile_pictures::user_profile_picture_updated_at.desc())
        .select(user_profile_pictures::user_profile_picture_link)
        .first::<Option<String>>(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
        .flatten();

    drop(conn);

    let country_map = state.country_map.read().await;
    let user_country_flag = country_map.get_flag_by_code(user_country);
    drop(country_map);

    let resp = PhotographCommentResponse::from_comment_votestate_and_badge_info(
        updated,
        vote_state,
        UserBadgeInfo {
            user_name,
            user_profile_picture_url: pic.unwrap_or_default(),
            user_country_flag,
        },
    );

    Ok(http_resp(resp, (), start))
}
