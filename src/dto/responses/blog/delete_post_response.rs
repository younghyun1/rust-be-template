use serde_derive::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct DeletePostResponse {
    pub deleted_post_id: Uuid,
}
