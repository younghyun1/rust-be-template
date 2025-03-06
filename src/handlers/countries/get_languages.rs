use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::response_data::http_resp, errors::code_error::HandlerResponse,
    init::state::ServerState, util::time::now::tokio_now,
};

pub async fn get_languages(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let languages_map_lock = state.languages_map.read().await;

    let serialized_map = languages_map_lock.serialized_map.clone();

    drop(languages_map_lock);

    Ok(http_resp(serialized_map, (), start))
}
