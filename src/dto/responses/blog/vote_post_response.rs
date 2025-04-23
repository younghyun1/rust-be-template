#[derive(serde_derive::Serialize)]
pub struct VotePostResponse {
    pub upvote_count: i64,
    pub downvote_count: i64,
}
