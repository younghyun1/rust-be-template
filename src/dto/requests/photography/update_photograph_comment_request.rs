use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct UpdatePhotographCommentRequest {
    pub comment_content: String,
}
