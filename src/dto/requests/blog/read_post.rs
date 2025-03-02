#[derive(serde_derive::Deserialize)]
pub struct ReadPostRequest {
    pub post_id: uuid::Uuid,
}
