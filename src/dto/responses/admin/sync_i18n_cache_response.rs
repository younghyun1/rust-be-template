use serde_derive::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct SyncI18nCacheResponse {
    pub num_rows: usize,
}
