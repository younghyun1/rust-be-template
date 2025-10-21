use serde_derive::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct DeletePostResponse {
    pub deleted_post_id: Uuid,
}
