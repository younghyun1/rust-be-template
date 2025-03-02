use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, QueryDsl};
use uuid::Uuid;

use diesel_async::RunQueryDsl;

use crate::{
    domain::blog::{NewPost, Post, PostInfo},
    dto::{
        requests::blog::submit_post_request::SubmitPostRequest,
        responses::{blog::submit_post_response::SubmitPostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::posts,
    util::{string::generate_slug::generate_slug, time::now::tokio_now},
};

// .route("/blog/submit-post", post(submit_post))
// #[derive(Deserialize)]
// pub struct SubmitPostRequest {
//     pub post_title: String,
//     pub post_content: String,
//     pub post_tags: Vec<String>,
//     pub post_is_published: bool,
// }
// is_published true for immediate publication
// is_published false for saving
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

    // Authentication logic remains the same
    let session_id: Uuid = match cookie_jar.get("session_id") {
        Some(session_id) => match session_id.value().parse::<Uuid>() {
            Ok(user_id) => user_id,
            Err(_) => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
        },
        None => return Err(CodeError::UNAUTHORIZED_ACCESS.into()),
    };

    let session: Session = state
        .get_session(&session_id)
        .await
        .map_err(|e| code_err(CodeError::UNAUTHORIZED_ACCESS, e))?;

    let user_id: Uuid = session.get_user_id();
    drop(session);

    // Generate slug (only for new posts or if title changed)
    let slug: String = generate_slug(&request.post_title);

    let post: Post = match request.post_id {
        // CASE: Editing an existing post
        Some(post_id) => {
            // First, verify the post exists and belongs to this user
            diesel::dsl::select(diesel::dsl::exists(
                posts::table
                    .filter(posts::post_id.eq(post_id))
                    .filter(posts::user_id.eq(user_id))
            ))
            .get_result::<bool>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
            .then_some(())
            .ok_or_else(|| code_err(CodeError::POST_NOT_FOUND, "Post not found or not owned by user"))?;

            // Update the existing post
            diesel::update(posts::table.filter(posts::post_id.eq(post_id)))
                .set((
                    posts::post_title.eq(&request.post_title),
                    posts::post_slug.eq(&slug),
                    posts::post_content.eq(&request.post_content),
                    posts::post_is_published.eq(request.post_is_published),
                    posts::post_updated_at.eq(chrono::Utc::now()),
                ))
                .returning(posts::all_columns)
                .get_result(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?
        }
        // CASE: Creating a new post
        None => {
            let new_post = NewPost::new(
                &user_id,
                &request.post_title,
                &slug,
                &request.post_content,
                request.post_is_published,
            );

            diesel::insert_into(posts::table)
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
                })?
        }
    };

    drop(conn);

    // Update cache
    let post_info: PostInfo = post.clone().into();
    state.insert_post_to_cache(&post_info).await;

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
