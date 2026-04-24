use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::{live_chat::LiveChatCacheStatsResponse, response_data::http_resp},
    errors::code_error::{CodeErrorResp, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/live-chat/cache-stats",
    tag = "live_chat",
    responses(
        (status = 200, description = "Live chat cache stats", body = LiveChatCacheStatsResponse),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_live_chat_cache_stats(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let stats = state.live_chat_cache.stats().await;

    Ok(http_resp(
        LiveChatCacheStatsResponse::from(stats),
        (),
        start,
    ))
}
