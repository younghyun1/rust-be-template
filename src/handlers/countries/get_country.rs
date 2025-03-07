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

#[derive(serde_derive::Deserialize)]
pub struct GetCountryQueryParams {
    country_id: i32,
}

pub async fn get_country(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<GetCountryQueryParams>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let country_map_lock = state.country_map.read().await;

    let country_id = *country_map_lock
        .by_id
        .get(&query.country_id)
        .ok_or("No country found by ID!")
        .map_err(|e| code_err(CodeError::COUNTRY_NOT_FOUND, e))?;

    let country_and_subdivisions = country_map_lock.rows[country_id].clone();

    drop(country_map_lock);

    Ok(http_resp(country_and_subdivisions, (), start))
}
