use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A single photograph item as exposed to API consumers.
///
/// This is intentionally decoupled from the DB `Photograph` struct so we can
/// change DB details without breaking the public API.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PhotographItem {
    pub photograph_id: Uuid,
    pub user_id: Uuid,
    pub photograph_shot_at: Option<DateTime<Utc>>,
    pub photograph_created_at: DateTime<Utc>,
    pub photograph_updated_at: DateTime<Utc>,
    pub photograph_image_type: i32,
    pub photograph_is_on_cloud: bool,
    pub photograph_link: String,
    pub photograph_comments: String,
    pub photograph_lat: f64,
    pub photograph_lon: f64,
    pub photograph_thumbnail_link: String,
}

/// Pagination metadata for list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    /// Current page number (1-based).
    pub page: i64,
    /// Page size requested/used.
    pub page_size: i64,
    /// Total number of items matching the query.
    pub total_items: i64,
    /// Total number of pages given `total_items` and `page_size`.
    pub total_pages: i64,
    /// Whether there is a next page after `page`.
    pub has_next: bool,
    /// Whether there is a previous page before `page`.
    pub has_prev: bool,
}

/// Response DTO for paginated photograph queries.
///
/// This is what `GET /photographs` should serialize as the `data` portion
/// of your `http_resp`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetPhotographsResponse {
    pub items: Vec<PhotographItem>,
    pub pagination: PaginationMeta,
}
