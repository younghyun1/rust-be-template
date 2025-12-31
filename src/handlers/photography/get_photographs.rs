use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};

use diesel::{ExpressionMethods, QueryDsl};

use diesel_async::RunQueryDsl;

use crate::{
    domain::photography::photographs::Photograph,
    dto::responses::photography::get_photograph_response::{
        GetPhotographsResponse, PaginationMeta, PhotographItem,
    },
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::photographs::dsl::*,
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/photographs/get",
    tag = "photography",
    params(
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("page_size" = Option<i64>, Query, description = "Items per page (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Successfully retrieved photographs", body = GetPhotographsResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_photographs(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Parse pagination parameters from query string.
    // ?page=1&page_size=20 by default
    let page: i64 = params
        .get("page")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|p| *p > 0)
        .unwrap_or(1);

    let page_size: i64 = params
        .get("page_size")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|s| *s > 0 && *s <= 100)
        .unwrap_or(20);

    let offset_val = (page - 1) * page_size;

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    // Get total count for pagination metadata
    let total_items: i64 = photographs
        .count()
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    // Fetch a single page of photographs ordered by most recently shot
    let results: Result<Vec<Photograph>, diesel::result::Error> = photographs
        .order(photograph_shot_at.desc())
        .offset(offset_val)
        .limit(page_size)
        .load::<Photograph>(&mut conn)
        .await;

    let photographs_vec = results.map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let total_pages = if total_items == 0 {
        0
    } else {
        ((total_items + page_size - 1) / page_size).max(1)
    };

    let pagination = PaginationMeta {
        page,
        page_size,
        total_items,
        total_pages,
        has_next: page < total_pages,
        has_prev: page > 1 && total_pages > 0,
    };

    let items: Vec<PhotographItem> = photographs_vec
        .into_iter()
        .map(|p| PhotographItem {
            photograph_id: p.photograph_id,
            user_id: p.user_id,
            photograph_shot_at: p.photograph_shot_at,
            photograph_created_at: p.photograph_created_at,
            photograph_updated_at: p.photograph_updated_at,
            photograph_image_type: p.photograph_image_type,
            photograph_is_on_cloud: p.photograph_is_on_cloud,
            photograph_link: p.photograph_link,
            photograph_comments: p.photograph_comments,
            photograph_lat: p.photograph_lat,
            photograph_lon: p.photograph_lon,
            photograph_thumbnail_link: p.photograph_thumbnail_link,
        })
        .collect();

    let response = GetPhotographsResponse { items, pagination };

    Ok(http_resp(response, (), start))
}
