use serde_derive::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct DeleteCommentResponse {
    pub deleted_comment_id: Uuid,
}
