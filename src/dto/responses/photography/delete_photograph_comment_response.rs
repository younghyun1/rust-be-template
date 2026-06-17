use serde_derive::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct DeletePhotographCommentResponse {
    pub deleted_photograph_comment_id: Uuid,
}
