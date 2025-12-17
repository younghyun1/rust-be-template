use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct UpdateCommentRequest {
    pub comment_content: String,
}
