use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[derive(serde::Deserialize)]
pub struct LanguageQuery {
    language_id: i32,
}

pub async fn get_language(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<LanguageQuery>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let languages_map_lock = state.languages_map.read().await;

    let language = languages_map_lock
        .lookup_by_code(query.language_id)
        .ok_or(())
        .map_err(|_| code_err(CodeError::LANGUAGE_NOT_FOUND, "Language not found!"))?;

    drop(languages_map_lock);

    Ok(http_resp(language, (), start))
}
