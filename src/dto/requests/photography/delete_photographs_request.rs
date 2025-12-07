use serde_derive::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DeletePhotographsRequest {
    pub photograph_ids: Vec<Uuid>,
}
