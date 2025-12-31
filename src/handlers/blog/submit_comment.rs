use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, prelude::Insertable};
use uuid::Uuid;

use diesel_async::RunQueryDsl;

use crate::{
    domain::blog::blog::{Comment as DbComment, CommentResponse, UserBadgeInfo, VoteState},
    dto::{
        requests::blog::submit_comment::SubmitCommentRequest, responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::{comments, user_profile_pictures, users},
    util::time::now::tokio_now,
};

// Insert the comment
#[derive(Insertable)]
#[diesel(table_name = comments)]
struct NewComment<'a> {
    pub post_id: &'a Uuid,
    pub user_id: &'a Uuid,
    pub comment_content: &'a str,
    pub parent_comment_id: Option<&'a Uuid>,
}

#[utoipa::path(
    post,
    path = "/api/blog/{post_id}/comment",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to comment on")
    ),
    request_body = SubmitCommentRequest,
    responses(
        (status = 200, description = "Comment submitted successfully", body = CommentResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn submit_comment(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
    Json(request): Json<SubmitCommentRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let user_id: Uuid = if request.is_guest {
        return Err(CodeError::UNAUTHORIZED_ACCESS.into());
    } else {
        // Get user id from session (same as submit_post)
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

        let uid: Uuid = session.get_user_id();
        drop(session);
        uid
    };

    let new_comment = NewComment {
        post_id: &post_id,
        user_id: &user_id,
        comment_content: &request.comment_content,
        parent_comment_id: request.parent_comment_id.as_ref(),
    };

    let inserted_comment: DbComment = diesel::insert_into(comments::table)
        .values(new_comment)
        .returning(comments::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

    let user_name: String = users::table
        .filter(users::user_id.eq(user_id))
        .select(users::user_name)
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let user_profile_picture_url: Option<String> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq(user_id))
        .order(user_profile_pictures::user_profile_picture_updated_at.desc())
        .select(user_profile_pictures::user_profile_picture_link)
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
        .flatten();

    drop(conn);

    let response = CommentResponse::from_comment_votestate_and_badge_info(
        inserted_comment,
        VoteState::DidNotVote,
        UserBadgeInfo {
            user_name,
            user_profile_picture_url: user_profile_picture_url.unwrap_or_default(),
        },
    );

    Ok(http_resp(response, (), start))
}
