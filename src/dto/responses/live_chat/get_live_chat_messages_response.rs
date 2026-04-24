use serde_derive::Serialize;
use utoipa::ToSchema;

use super::live_chat_message_response::LiveChatMessageItem;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetLiveChatMessagesResponse {
    pub items: Vec<LiveChatMessageItem>,
    pub next_before_message_id: Option<uuid::Uuid>,
    pub has_more: bool,
}
