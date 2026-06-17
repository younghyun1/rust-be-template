use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct VotePhotographRequest {
    pub is_upvote: bool,
}
