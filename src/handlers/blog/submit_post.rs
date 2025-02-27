use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use diesel_async::RunQueryDsl;

use crate::{
    domain::blog::{NewPost, Post},
    dto::{
        requests::blog::submit_post_request::SubmitPostRequest,
        responses::{blog::submit_post_response::SubmitPostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::posts,
    util::{string::generate_slug::generate_slug, time::now::tokio_now},
};

pub async fn submit_post(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SubmitPostRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Get user_id out of JSON session.
    let session_id: Uuid = match cookie_jar.get("session_id") {
        Some(session_id) => match session_id.value().parse::<Uuid>() {
            Ok(user_id) => user_id,
            Err(_) => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
        },
        None => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
    };

    // TODO: Consider new error code here
    let session: Session = state
        .get_session(&session_id)
        .await
        .map_err(|e| code_err(CodeError::UNAUTHORIZED_ACCESS, e))?;

    let user_id: Uuid = session.get_user_id();
    let slug: String = generate_slug(&request.post_title);
    drop(session);

    let new_post = NewPost::new(
        &user_id,
        &request.post_title,
        &slug,
        &request.post_content,
        request.post_is_published,
    );

    let post: Post = diesel::insert_into(posts::table)
        .values(new_post)
        .returning(posts::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => code_err(CodeError::POST_TITLE_NOT_UNIQUE, e),
            _ => code_err(CodeError::DB_INSERTION_ERROR, e),
        })?;

    drop(conn);

    Ok(http_resp(
        SubmitPostResponse {
            user_id,
            post_title: post.post_title,
            post_slug: post.post_slug,
            post_created_at: post.post_created_at,
            post_updated_at: post.post_updated_at,
            post_is_published: post.post_is_published,
        },
        (),
        start,
    ))
}
