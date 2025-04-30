use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};

use crate::{
    domain::i18n::i18n::InternationalizationString,
    dto::{
        requests::i18n::get_country_language_bundle_request::GetCountryLanguageBundleRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

pub async fn get_country_language_bundle(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<GetCountryLanguageBundleRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let bundle: Vec<u8> = InternationalizationString::get_country_language_bundle_from_cache(
        request.country_code,
        request.language_code,
        &state,
    )
    .await
    .map_err(|e| code_err(CodeError::COULD_NOT_GET_I18N_BUNDLE, e))?;

    Ok(http_resp(bundle, (), start))
}
