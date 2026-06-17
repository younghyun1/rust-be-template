use utoipa::ToSchema;
use uuid::Uuid;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct SubmitPhotographCommentRequest {
    pub parent_comment_id: Option<Uuid>,
    pub comment_content: String,
}
