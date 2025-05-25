use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use diesel::prelude::Insertable;
use uuid::Uuid;

use diesel_async::RunQueryDsl;

use crate::{
    domain::blog::blog::Comment as DbComment,
    dto::{
        requests::blog::submit_comment::SubmitCommentRequest, responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::comments,
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

    drop(conn);

    Ok(http_resp(inserted_comment, (), start))
}
