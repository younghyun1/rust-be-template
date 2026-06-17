//! `POST /api/photographs/{photograph_id}/comment` — add a (possibly threaded)
//! comment. Protected tier: any authenticated user.

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
        blog::blog::{UserBadgeInfo, VoteState},
        photography::social::{NewPhotographComment, PhotographComment, PhotographCommentResponse},
    },
    dto::{
        requests::photography::submit_photograph_comment_request::SubmitPhotographCommentRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{photograph_comments, user_profile_pictures, users},
    util::time::now::tokio_now,
};

#[utoipa::path(
    post,
    path = "/api/photographs/{photograph_id}/comment",
    tag = "photography",
    params(("photograph_id" = Uuid, Path, description = "Photograph to comment on")),
    request_body = SubmitPhotographCommentRequest,
    responses(
        (status = 200, description = "Comment created", body = PhotographCommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn submit_photograph_comment(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(photograph_id): Path<Uuid>,
    Json(request): Json<SubmitPhotographCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    if request.comment_content.trim().is_empty() {
        return Err(code_err(
            CodeError::INVALID_REQUEST,
            "Comment content must not be empty",
        ));
    }

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let new_comment = NewPhotographComment {
        photograph_id: &photograph_id,
        user_id: &user_id,
        photograph_comment_content: &request.comment_content,
        parent_photograph_comment_id: request.parent_comment_id.as_ref(),
    };

    let inserted: PhotographComment = diesel::insert_into(photograph_comments::table)
        .values(new_comment)
        .returning(photograph_comments::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

    // Author badge (the comment author is the caller).
    let user_row: Option<(String, i32)> = users::table
        .filter(users::user_id.eq(user_id))
        .select((users::user_name, users::user_country))
        .first::<(String, i32)>(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;
    let (user_name, user_country) = user_row.unwrap_or_else(|| ("Unknown".to_string(), 0));

    let pic: Option<String> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq(user_id))
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
        inserted,
        VoteState::DidNotVote,
        UserBadgeInfo {
            user_name,
            user_profile_picture_url: pic.unwrap_or_default(),
            user_country_flag,
        },
    );

    Ok(http_resp(resp, (), start))
}
