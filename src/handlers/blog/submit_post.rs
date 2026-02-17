use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::{Json, extract::State, response::IntoResponse};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, QueryDsl};
use uuid::Uuid;

use diesel_async::RunQueryDsl;

use crate::{
    domain::blog::blog::{CachedPostInfo, NewPost, NewPostTag, NewTag, Post, PostInfo},
    dto::{
        requests::blog::submit_post_request::SubmitPostRequest,
        responses::{blog::submit_post_response::SubmitPostResponse, response_data::http_resp},
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::{ServerState, Session},
    schema::{post_tags, posts, tags},
    util::{
        auth::is_superuser::is_superuser, string::generate_slug::generate_slug,
        time::now::tokio_now,
    },
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
#[utoipa::path(
    post,
    path = "/api/blog/posts",
    tag = "blog",
    request_body = SubmitPostRequest,
    responses(
        (status = 200, description = "Post submitted or updated", body = SubmitPostResponse),
        (status = 401, description = "Unauthorized access", body = CodeErrorResp),
        (status = 403, description = "Forbidden access", body = CodeErrorResp),
        (status = 404, description = "Post not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn submit_post(
    cookie_jar: CookieJar,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SubmitPostRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Normalize + deduplicate requested tags once and reuse across compare/persist/cache.
    let mut seen_tags: HashSet<String> = HashSet::new();
    let requested_tags: Vec<String> = request
        .post_tags
        .iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen_tags.insert(tag.clone()))
        .collect();

    let cached_tags: Option<Vec<String>> = if let Some(post_id) = request.post_id {
        state
            .get_post_from_cache(&post_id)
            .await
            .map(|cached| cached.post_tags)
    } else {
        None
    };

    let tags_changed: bool = match (request.post_id, &cached_tags) {
        (None, _) => true,
        (Some(_), Some(current_tags)) => {
            let current_tag_set: HashSet<String> = current_tags
                .iter()
                .map(|tag| tag.trim().to_lowercase())
                .filter(|tag| !tag.is_empty())
                .collect();
            let requested_tag_set: HashSet<String> = requested_tags.iter().cloned().collect();
            current_tag_set != requested_tag_set
        }
        (Some(_), None) => true,
    };

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Authentication logic remains the same
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
            "User is not authorized to submit posts",
        ));
    }

    // Generate slug (only for new posts or if title changed)
    let slug: String = generate_slug(&request.post_title);
    let now = chrono::Utc::now();
    let rendered_markdown: String =
        comrak::markdown_to_html(&request.post_content, &comrak::Options::default());
    let post_metadata = serde_json::json!({
        "markdown_content": request.post_content
    });

    let post: Post = match request.post_id {
        // CASE: Editing an existing post
        Some(post_id) => {
            // First, verify the post exists and belongs to this user
            diesel::dsl::select(diesel::dsl::exists(
                posts::table
                    .filter(posts::post_id.eq(post_id))
                    .filter(posts::user_id.eq(user_id)),
            ))
            .get_result::<bool>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
            .then_some(())
            .ok_or_else(|| {
                code_err(
                    CodeError::POST_NOT_FOUND,
                    "Post not found or not owned by user",
                )
            })?;

            let existing_published_at: Option<chrono::DateTime<chrono::Utc>> = posts::table
                .filter(posts::post_id.eq(post_id))
                .filter(posts::user_id.eq(user_id))
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
            diesel::update(posts::table.filter(posts::post_id.eq(post_id)))
                .set((
                    posts::post_title.eq(&request.post_title),
                    posts::post_slug.eq(&slug),
                    posts::post_content.eq(&rendered_markdown),
                    posts::post_is_published.eq(request.post_is_published),
                    posts::post_published_at.eq(new_published_at),
                    posts::post_updated_at.eq(chrono::Utc::now()),
                    posts::post_metadata.eq(&post_metadata),
                ))
                .returning(posts::all_columns)
                .get_result(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_UPDATE_ERROR, e))?
        }
        // CASE: Creating a new post
        None => {
            let new_published_at = if request.post_is_published {
                Some(now)
            } else {
                None
            };
            let new_post = NewPost::new(
                &user_id,
                &request.post_title,
                &slug,
                &rendered_markdown,
                new_published_at,
                request.post_is_published,
                &post_metadata,
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

    if tags_changed {
        // Replace post<->tag relations only when effective tags changed.
        diesel::delete(post_tags::table.filter(post_tags::post_id.eq(post.post_id)))
            .execute(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_DELETION_ERROR, e))?;

        if !requested_tags.is_empty() {
            let new_tags: Vec<NewTag<'_>> =
                requested_tags.iter().map(|tag| NewTag::new(tag)).collect();

            diesel::insert_into(tags::table)
                .values(&new_tags)
                .on_conflict(tags::tag_name)
                .do_nothing()
                .execute(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;

            let tag_rows: Vec<(i16, String)> = tags::table
                .filter(tags::tag_name.eq_any(&requested_tags))
                .select((tags::tag_id, tags::tag_name))
                .load(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

            let tag_id_by_name: HashMap<String, i16> =
                tag_rows.into_iter().map(|(id, name)| (name, id)).collect();

            let tag_ids: Vec<i16> = requested_tags
                .iter()
                .map(|tag| {
                    tag_id_by_name.get(tag).copied().ok_or_else(|| {
                        code_err(
                            CodeError::DB_QUERY_ERROR,
                            format!("Tag ID not found after upsert for tag '{tag}'"),
                        )
                    })
                })
                .collect::<Result<Vec<i16>, CodeErrorResp>>()?;

            let new_post_tags: Vec<NewPostTag> = tag_ids
                .iter()
                .map(|tag_id| NewPostTag::new(&post.post_id, tag_id))
                .collect();

            diesel::insert_into(post_tags::table)
                .values(&new_post_tags)
                .execute(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_INSERTION_ERROR, e))?;
        }
    }

    drop(conn);

    let final_tags: Vec<String> = if tags_changed {
        requested_tags.clone()
    } else {
        cached_tags.unwrap_or_else(|| requested_tags.clone())
    };

    let post_info: PostInfo = post.clone().into();
    let cached_post = CachedPostInfo::from_post_info_with_tags(post_info, final_tags.clone());
    state.insert_post_to_cache(&cached_post).await;

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
