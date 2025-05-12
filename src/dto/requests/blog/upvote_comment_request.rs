use uuid::Uuid;

#[derive(serde_derive::Deserialize)]
pub struct UpvoteCommentRequest {
    pub comment_id: Uuid,
    pub is_upvote: bool,
}
