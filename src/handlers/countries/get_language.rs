use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    domain::country::IsoLanguage,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/dropdown/language/{language_id}",
    params(
        ("language_id" = i32, Path, description = "ID of the language to retrieve")
    ),
    responses(
        (status = 200, description = "Language information", body = IsoLanguage),
        (status = 404, description = "Language not found", body = CodeErrorResp)
    )
)]
pub async fn get_language(
    State(state): State<Arc<ServerState>>,
    Path(language_id): Path<i32>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let languages_map_lock = state.languages_map.read().await;

    let language = languages_map_lock
        .lookup_by_code(language_id)
        .ok_or(())
        .map_err(|_| code_err(CodeError::LANGUAGE_NOT_FOUND, "Language not found!"))?;

    drop(languages_map_lock);

    Ok(http_resp(language, (), start))
}
