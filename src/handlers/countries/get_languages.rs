use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    domain::country::IsoLanguage,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeErrorResp, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/dropdown/language",
    responses(
        (status = 200, description = "List of languages", body = [IsoLanguage]),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_languages(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let languages_map_lock = state.languages_map.read().await;

    let languages_list: Vec<IsoLanguage> = languages_map_lock.rows.clone();

    drop(languages_map_lock);

    Ok(http_resp(languages_list, (), start))
}
