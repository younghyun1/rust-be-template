use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SubmitCommentRequest {
    pub is_guest: bool,
    pub guest_id: Option<String>,
    pub guest_password: Option<String>,
    pub parent_comment_id: Option<uuid::Uuid>,
    pub comment_content: String,
}
