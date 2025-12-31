use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct UpvoteCommentRequest {
    pub is_upvote: bool,
}
