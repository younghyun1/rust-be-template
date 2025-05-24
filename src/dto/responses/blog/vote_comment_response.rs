#[derive(serde::Serialize)]
pub struct VoteCommentResponse {
    pub upvote_count: i64,
    pub downvote_count: i64,
    pub is_upvote: bool,
}
