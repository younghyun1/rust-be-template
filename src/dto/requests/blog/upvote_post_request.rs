use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct UpvotePostRequest {
    pub is_upvote: bool,
}
