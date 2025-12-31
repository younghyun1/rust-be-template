use serde_derive::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct SubmitPostRequest {
    pub post_id: Option<Uuid>,
    pub post_title: String,
    pub post_content: String,
    pub post_tags: Vec<String>,
    pub post_is_published: bool,
}
