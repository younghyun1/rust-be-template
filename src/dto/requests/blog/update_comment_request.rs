use serde_derive::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct UpdateCommentRequest {
    pub comment_content: String,
}
