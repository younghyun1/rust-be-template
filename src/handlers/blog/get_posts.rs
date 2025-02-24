use std::sync::Arc;

use crate::{
    domain::blog::PostInfo,
    dto::{
        requests::blog::get_posts_request::GetPostsRequest,
        responses::{blog::get_posts::GetPostsResponse, response_data::http_resp},
    },
    errors::code_error::HandlerResponse,
    init::state::ServerState,
    util::time::now::tokio_now,
};
use axum::{Json, extract::State, response::IntoResponse};

pub async fn get_posts(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<GetPostsRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let (post_infos, available_pages): (Vec<PostInfo>, usize) = state
        .get_posts_from_cache(request.page, request.posts_per_page)
        .await;

    Ok(http_resp(
        GetPostsResponse {
            posts: post_infos,
            available_pages,
        },
        (),
        start,
    ))
}
