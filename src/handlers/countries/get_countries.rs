use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::{
    dto::responses::response_data::http_resp, errors::code_error::HandlerResponse,
    init::state::ServerState, util::time::now::tokio_now,
};

pub async fn get_countries(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let country_table_lock = state.country_map.read().await;

    let countries = country_table_lock.serialized_country_list.clone();

    drop(country_table_lock);

    Ok(http_resp(countries, (), start))
}
