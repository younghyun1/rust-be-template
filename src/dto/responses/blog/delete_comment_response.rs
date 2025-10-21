use serde_derive::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct DeleteCommentResponse {
    pub deleted_comment_id: Uuid,
}
