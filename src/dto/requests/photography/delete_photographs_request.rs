use serde_derive::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct DeletePhotographsRequest {
    pub photograph_ids: Vec<Uuid>,
}
