use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct SubmitPostRequest {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}
