use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    domain::country::IsoCountry,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeErrorResp, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/dropdown/country",
    responses(
        (status = 200, description = "List of countries", body = [IsoCountry]),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_countries(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let country_table_lock = state.country_map.read().await;

    let countries = country_table_lock.serialized_country_list.clone();

    drop(country_table_lock);

    Ok(http_resp(countries, (), start))
}
