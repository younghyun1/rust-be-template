use crate::{
    domain::country::IsoCountrySubdivision,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse},
    init::state::ServerState,
    util::time::now::tokio_now,
};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use std::sync::Arc;

#[utoipa::path(
    get,
    path = "/api/dropdown/country/{country_id}/subdivision",
    params(
        ("country_id" = i32, Path, description = "ID of the country to retrieve subdivisions for")
    ),
    responses(
        (status = 200, description = "List of subdivisions for the country", body = [IsoCountrySubdivision]),
        (status = 404, description = "Country not found", body = CodeErrorResp)
    )
)]
pub async fn get_subdivisions_for_country(
    State(state): State<Arc<ServerState>>,
    Path(country_id): Path<i32>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let country_map_lock = state.country_map.read().await;

    let subdivisions = match country_map_lock.by_id.get(&country_id) {
        Some(id) => country_map_lock.rows[*id].subdivisions.clone(),
        None => return Err(CodeError::COUNTRY_NOT_FOUND.into()),
    };

    drop(country_map_lock);

    Ok(http_resp(subdivisions, (), start))
}
