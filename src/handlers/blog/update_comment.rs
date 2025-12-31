use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::blog::{Comment as DbComment, CommentResponse, UserBadgeInfo, VoteState},
    dto::{
        requests::blog::update_comment_request::UpdateCommentRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::{comments, user_profile_pictures, users},
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

#[utoipa::path(
    patch,
    path = "/api/blog/{post_id}/{comment_id}",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post"),
        ("comment_id" = Uuid, Path, description = "ID of the comment to update")
    ),
    request_body = UpdateCommentRequest,
    responses(
        (status = 200, description = "Comment updated successfully", body = CommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden", body = CodeErrorResp),
        (status = 404, description = "Comment not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn update_comment(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
    Path((_post_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Authentication
    let session_id: Uuid = match cookie_jar.get("session_id") {
        Some(session_id) => match session_id.value().parse::<Uuid>() {
            Ok(session_id) => session_id,
            Err(_) => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
        },
        None => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
    };

    let session: Session = state
        .get_session(&session_id)
        .await
        .map_err(|e| code_err(CodeError::UNAUTHORIZED_ACCESS, e))?;

    let requester_id: Uuid = session.get_user_id();
    drop(session);

    let is_superuser: bool = match is_superuser(state.clone(), requester_id).await {
        Ok(is_superuser) => is_superuser,
        Err(e) => return Err(code_err(CodeError::DB_QUERY_ERROR, e)),
    };

    // Check authorship
    let author_id: Uuid = comments::table
        .select(comments::user_id)
        .filter(comments::comment_id.eq(comment_id))
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    if author_id != requester_id && !is_superuser {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "User is not authorized to edit this comment",
        ));
    }

    // Update comment
    let updated_comment: DbComment =
        diesel::update(comments::table.filter(comments::comment_id.eq(comment_id)))
            .set((
                comments::comment_content.eq(&request.comment_content),
                comments::comment_updated_at.eq(chrono::Utc::now()),
            ))
            .returning(comments::all_columns)
            .get_result(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?;

    // Get user info for response
    let user_name: String = users::table
        .filter(users::user_id.eq(author_id))
        .select(users::user_name)
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let user_profile_picture_url: Option<String> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq(author_id))
        .order(user_profile_pictures::user_profile_picture_updated_at.desc())
        .select(user_profile_pictures::user_profile_picture_link)
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
        .flatten();

    drop(conn);

    Ok(http_resp(
        CommentResponse::from_comment_votestate_and_badge_info(
            updated_comment,
            VoteState::DidNotVote,
            UserBadgeInfo {
                user_name,
                user_profile_picture_url: user_profile_picture_url.unwrap_or_default(),
            },
        ),
        (),
        start,
    ))
}
