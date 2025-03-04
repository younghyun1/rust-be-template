use crate::domain::blog::{Comment, Post};

#[derive(serde_derive::Serialize)]
pub struct ReadPostResponse {
    pub post: Post,
    pub comments: Vec<Comment>,
}
