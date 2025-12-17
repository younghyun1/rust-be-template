use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct UpdatePostRequest {
    pub post_title: String,
    pub post_content: String,
    pub post_tags: Vec<String>,
    pub post_is_published: bool,
}
