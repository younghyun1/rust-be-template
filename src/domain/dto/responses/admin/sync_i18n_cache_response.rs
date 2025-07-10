use serde_derive::Serialize;

#[derive(Serialize)]
pub struct SyncI18nCacheResponse {
    pub num_rows: usize,
}
