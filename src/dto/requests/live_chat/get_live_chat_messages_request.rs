use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, serde_derive::Deserialize, ToSchema)]
#[serde(default = "GetLiveChatMessagesRequest::default")]
pub struct GetLiveChatMessagesRequest {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub before_message_id: Option<Uuid>,
}

impl Default for GetLiveChatMessagesRequest {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            before_message_id: None,
        }
    }
}

#[inline(always)]
fn default_limit() -> usize {
    50
}
