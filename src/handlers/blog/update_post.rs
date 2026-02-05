use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::blog::blog::{CachedPostInfo, Post, PostInfo},
    dto::{
        requests::blog::update_post_request::UpdatePostRequest,
        responses::{blog::submit_post_response::SubmitPostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::posts,
    util::{
        auth::is_superuser::is_superuser, string::generate_slug::generate_slug,
        time::now::tokio_now,
    },
};

#[utoipa::path(
    patch,
    path = "/api/blog/{post_id}",
    tag = "blog",
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to update")
    ),
    request_body = UpdatePostRequest,
    responses(
        (status = 200, description = "Post updated successfully", body = SubmitPostResponse),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden", body = CodeErrorResp),
        (status = 404, description = "Post not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn update_post(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
    Path(post_id): Path<Uuid>,
    Json(request): Json<UpdatePostRequest>,
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

    let user_id: Uuid = session.get_user_id();
    drop(session);

    let is_superuser = match is_superuser(state.clone(), user_id).await {
        Ok(is_superuser) => is_superuser,
        Err(e) => return Err(code_err(CodeError::DB_QUERY_ERROR, e)),
    };

    if !is_superuser {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "User is not authorized to edit posts",
        ));
    }

    // Generate slug from title
    let slug: String = generate_slug(&request.post_title);
    let now = chrono::Utc::now();
    let rendered_markdown: String =
        comrak::markdown_to_html(&request.post_content, &comrak::Options::default());

    let post_metadata = serde_json::json!({
        "markdown_content": request.post_content
    });

    let existing_published_at: Option<chrono::DateTime<chrono::Utc>> = posts::table
        .filter(posts::post_id.eq(post_id))
        .select(posts::post_published_at)
        .first::<Option<chrono::DateTime<chrono::Utc>>>(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let new_published_at = if request.post_is_published {
        existing_published_at.or(Some(now))
    } else {
        None
    };

    // Update the existing post
    let post: Post = diesel::update(posts::table.filter(posts::post_id.eq(post_id)))
        .set((
            posts::post_title.eq(&request.post_title),
            posts::post_slug.eq(&slug),
            posts::post_content.eq(&rendered_markdown),
            posts::post_is_published.eq(request.post_is_published),
            posts::post_published_at.eq(new_published_at),
            posts::post_updated_at.eq(now),
            posts::post_metadata.eq(&post_metadata),
        ))
        .returning(posts::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?;

    drop(conn);

    // TODO: Update tags in DB

    // Update cache
    if state
        .blog_posts_cache
        .update_async(&post.post_id, |_, cached| {
            cached.post_title = post.post_title.clone();
            cached.post_slug = post.post_slug.clone();
            cached.post_summary = post.post_summary.clone();
            cached.post_updated_at = post.post_updated_at;
            cached.post_published_at = post.post_published_at;
            cached.post_is_published = post.post_is_published;
            cached.post_view_count = post.post_view_count;
            cached.post_share_count = post.post_share_count;
        })
        .await
        .is_none()
    {
        let post_info = PostInfo::from(post.clone());
        // Use empty tags since update_post doesn't handle tags
        let cached_post = CachedPostInfo::from_post_info_with_tags(post_info, vec![]);
        state.insert_post_to_cache(&cached_post).await;
    }

    Ok(http_resp(
        SubmitPostResponse {
            post_id: post.post_id,
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
