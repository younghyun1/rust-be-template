use serde_derive::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct UpdatePostRequest {
    pub post_title: String,
    pub post_content: String,
    pub post_tags: Vec<String>,
    pub post_is_published: bool,
}
