use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use std::sync::Arc;

#[derive(serde_derive::Deserialize)]
pub struct GetSubdivisionsForCountryQueryParams {
    country_id: i32,
}

pub async fn get_subdivisions_for_country(
    State(state): State<Arc<ServerState>>,
    Query(query_params): Query<GetSubdivisionsForCountryQueryParams>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let country_map_lock = state.country_map.read().await;

    let subdivisions = match country_map_lock.by_id.get(&query_params.country_id) {
        Some(id) => country_map_lock.rows[*id].subdivisions.clone(),
        None => return Err(CodeError::COUNTRY_NOT_FOUND.into()),
    };

    drop(country_map_lock);

    Ok(http_resp(subdivisions, (), start))
}
