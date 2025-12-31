use utoipa::ToSchema;

#[derive(serde_derive::Serialize, ToSchema)]
pub struct VotePostResponse {
    pub upvote_count: i64,
    pub downvote_count: i64,
    pub is_upvote: bool,
}
