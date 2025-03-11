use uuid::Uuid;

#[derive(serde_derive::Deserialize)]
pub struct UpvotePostRequest {
    pub post_id: Uuid,
}
