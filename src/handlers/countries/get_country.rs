use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    domain::country::CountryAndSubdivisions,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/dropdown/country/{country_id}",
    params(
        ("country_id" = i32, Path, description = "ID of the country to retrieve")
    ),
    responses(
        (status = 200, description = "Country and its subdivisions", body = CountryAndSubdivisions),
        (status = 404, description = "Country not found", body = CodeErrorResp)
    )
)]
pub async fn get_country(
    State(state): State<Arc<ServerState>>,
    Path(country_id): Path<i32>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let country_map_lock = state.country_map.read().await;

    let country_id = *country_map_lock
        .by_id
        .get(&country_id)
        .ok_or("No country found by ID!")
        .map_err(|e| code_err(CodeError::COUNTRY_NOT_FOUND, e))?;

    let country_and_subdivisions = country_map_lock.rows[country_id].clone();

    drop(country_map_lock);

    Ok(http_resp(country_and_subdivisions, (), start))
}
