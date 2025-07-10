use crate::domain::blog::blog::{CommentResponse, Post, VoteState};

#[derive(serde_derive::Serialize)]
pub struct ReadPostResponse {
    pub post: Post,
    pub comments: Vec<CommentResponse>,
    pub vote_state: VoteState,
}
